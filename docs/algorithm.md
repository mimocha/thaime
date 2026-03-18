# Algorithm

This document explains the internals of the THAIME engine: how it turns Latin keystrokes into ranked Thai candidates. Each section builds on the previous one, following the data flow from keystroke to committed text.

## Dictionary & Trie Lookup

**Source:** `crates/thaime_engine/src/trie.rs`

### Double-Array Trie

THAIME uses a [double-array trie](https://en.wikipedia.org/wiki/Double-array_trie) (via the [yada](https://crates.io/crates/yada) crate) for dictionary lookup. This data structure is chosen for:

- **Fast prefix search** — `O(key_length)` per lookup, critical since we search at every position in the input
- **Compact representation** — the double-array encoding is more memory-efficient than a naive trie
- **Common prefix search** — yada's `common_prefix_search` returns all keys that are prefixes of the input in a single pass

### Data Model

The dictionary has three layers:

```
Romanization Key    Group ID    Word IDs        Word Metadata
────────────────    ────────    ────────        ─────────────
"ma"          ───►  group 0 ──► [5]        ──►  { thai: "มา",   freq: 0.008 }
"maa"         ───►  group 1 ──► [5]        ──►  { thai: "มา",   freq: 0.008 }
"maai"        ───►  group 2 ──► [0]        ──►  { thai: "ไม่",  freq: 0.013 }
"mai"         ───►  group 3 ──► [0, 2, 4]  ──►  { thai: "ไม่",  freq: 0.013 }
                                                 { thai: "ไหม",  freq: 0.005 }
                                                 { thai: "ใหม่", freq: 0.004 }
"nai"         ───►  group 4 ──► [1]        ──►  { thai: "ใน",   freq: 0.012 }
"sawatdee"    ───►  group 5 ──► [3]        ──►  { thai: "สวัสดี", freq: 0.003 }
```

- **Trie**: maps romanization byte strings to group IDs
- **Posting lists** (CSR format): maps group IDs to arrays of word IDs
- **Metadata table**: maps word IDs to Thai text and frequency

### CSR (Compressed Sparse Row) Posting Lists

Multiple romanization keys can map to the same word (e.g., "ma" and "maa" both map to "มา"), and one romanization key can map to multiple words (e.g., "mai" maps to "ไม่", "ไหม", "ใหม่"). The posting lists use CSR encoding:

```
group_offsets:  [0, 1, 2, 3, 6, 7, 8]
group_word_ids: [5, 5, 0, 0, 2, 4, 1, 3]
                 ^  ^  ^  ^-----^  ^  ^
                 │  │  │  group 3  │  │
                 │  │  group 2     │  group 5
                 │  group 1        group 4
                 group 0

To get word IDs for group g:
    start = group_offsets[g]
    end   = group_offsets[g + 1]
    word_ids = group_word_ids[start..end]
```

### Prefix Search

Given an input string, the trie returns all romanization keys that are prefixes of that string. This is the foundation of lattice construction.

**Example:** Input `"mainai"`, searching from position 0:

```
prefix_search("mainai")
    → "ma"  (len 2) → [มา]
    → "mai" (len 3) → [ไม่, ไหม, ใหม่]

prefix_search("inai")   ← from position 2
    → (no matches)

prefix_search("nai")    ← from position 3
    → "nai" (len 3) → [ใน]
```

The engine runs `prefix_search` at every byte position in the input (0 through n-1), collecting all possible word spans. This set of spans forms the word lattice.

## Word Lattice Construction

**Source:** `crates/thaime_engine/src/ranking.rs`

### From Prefix Matches to DAG

The prefix matches from all positions form a directed acyclic graph (DAG) — the **word lattice**. Each edge represents one possible word spanning `[start, end)` in the input.

**Example:** Input `"mainai"` (6 bytes)

```
Position: 0    1    2    3    4    5    6
          m    a    i    n    a    i
          │              │              │
          ├──── มา ──────┤              │    "ma" (0→2)
          │              │              │
          ├─── ไม่ ──────────┤          │    "mai" (0→3)
          ├─── ไหม ─────────┤          │    "mai" (0→3)
          ├─── ใหม่ ────────┤          │    "mai" (0→3)
          │              │   │          │
          │              │   ├── ใน ────┤    "nai" (3→6)
          │              │              │
          0              2   3          6
```

Edges are grouped by their **end position** for efficient Viterbi processing. Each edge carries:

- `start`, `end` — byte positions in the input
- `thai` — the Thai text for this word
- `word_id` — dictionary word ID (for n-gram lookup)
- `frequency` — raw word frequency from the dictionary
- `cost` — `-ln(max(freq, MIN_FREQ)) + LAMBDA`

### What Makes a Valid Path?

A **valid path** (complete tiling) must cover the entire input with no gaps and no overlaps. In the example above:

| Path | Words | Valid? |
|------|-------|--------|
| `มา` (0→2) + `ใน` (3→6) | 2 | No — gap at position 2→3 |
| `ไม่` (0→3) + `ใน` (3→6) | 2 | Yes |
| `ไหม` (0→3) + `ใน` (3→6) | 2 | Yes |
| `ใหม่` (0→3) + `ใน` (3→6) | 2 | Yes |

Only paths where edges chain end-to-start from position 0 to position n are valid candidates.

## Viterbi k-Best Ranking

**Source:** `crates/thaime_engine/src/ranking.rs`

### Overview

The engine uses a **Viterbi dynamic programming** algorithm to find the k-best paths through the word lattice. This is the same family of algorithms used in speech recognition and machine translation for finding optimal word sequences.

### Scoring Formula

Each lattice edge has a cost composed of three parts:

```
unigram_cost = -ln(max(freq, MIN_FREQ)) + LAMBDA
ngram_bonus  = ngram_weight * -ln(stupid_backoff_score)
edge_cost    = unigram_cost + ngram_bonus
path_cost    = Σ edge_costs
```

Lower cost = better candidate.

**Components:**

| Term | What it does |
|------|-------------|
| `-ln(freq)` | Frequent words get lower cost (better) |
| `LAMBDA` | Segmentation penalty per word — penalizes many short words |
| `ngram_bonus` | Context bonus — rewards sequences that match the language model |
| `MIN_FREQ` | Floor to avoid `-ln(0)` for rare words |

When n-gram data is not loaded, `ngram_bonus` is 0 and scoring reduces to unigram-only.

### Forward Pass

The Viterbi forward pass processes lattice positions left to right:

```
for end in 1..=n:
    for each edge ending at `end`:
        for each partial path at edge.start:
            new_cost = path.cost + edge_cost(edge, path.context)
            extend path with this edge
            store at position `end`
```

### State Key: Trigram Context

With n-gram scoring, the Viterbi state includes the **previous two words** (for trigram context):

```
State = (prev_thai_2, prev_thai_1)
```

At position 0, the state is seeded with the committed context from previous user input (up to 2 words). Different 2-word histories at the same position maintain separate path sets.

### Beam Pruning

To keep computation tractable, two levels of pruning are applied at each lattice position:

1. **Per-state top-k**: For each unique `(prev_thai_2, prev_thai_1)` state, keep only the `k` best partial paths
2. **Global beam**: Across all states at a position, keep at most `k * BEAM_MULTIPLIER` total paths

### Path Extraction

After the forward pass, all complete paths at position `n` are collected, sorted by cost, deduplicated by Thai text (keeping the lowest-cost instance), and truncated to `k` results.

### Worked Example

**Input:** `"sawatdee"` with the test dictionary.

```
Position: 0                              8
          s  a  w  a  t  d  e  e
          │                              │
          ├──── สวัสดี ──────────────────┤    "sawatdee" (0→8)
          │                              │
          0                              8

Lattice has 1 edge: สวัสดี (freq=0.003)

best[0] = { (None, None): [PartialPath { cost: 0.0, words: [] }] }

Processing end=8, edge สวัสดี (0→8):
    unigram_cost = -ln(0.003) + 1.0 = 5.81 + 1.0 = 6.81
    ngram_bonus  = 0.0 (no n-gram data)
    edge_cost    = 6.81

best[8] = { (None, "สวัสดี"): [PartialPath { cost: 6.81 }] }

Result: [Candidate { thai: "สวัสดี", score: 6.81, words: 1 }]
```

**Input:** `"mainai"` — a multi-word example.

```
best[0] = { (None, None): [cost: 0.0] }

end=2: edge มา (0→2, freq=0.008)
    cost = -ln(0.008) + 1.0 = 5.83
    best[2] = { (None, "มา"): [cost: 5.83] }

end=3: edges ไม่,ไหม,ใหม่ (0→3)
    ไม่: cost = -ln(0.013) + 1.0 = 5.34
    ไหม: cost = -ln(0.005) + 1.0 = 6.30
    ใหม่: cost = -ln(0.004) + 1.0 = 6.52
    best[3] = { (None,"ไม่"): [5.34], (None,"ไหม"): [6.30], (None,"ใหม่"): [6.52] }

end=6: edge ใน (3→6, freq=0.012)
    From (None,"ไม่") at pos 3: cost = 5.34 + (-ln(0.012) + 1.0) = 5.34 + 5.42 = 10.76
    From (None,"ไหม") at pos 3: cost = 6.30 + 5.42 = 11.72
    From (None,"ใหม่") at pos 3: cost = 6.52 + 5.42 = 11.94

Results (sorted by cost):
    1. ไม่ใน   (10.76) — 2 words
    2. ไหมใน   (11.72) — 2 words
    3. ใหม่ใน  (11.94) — 2 words
```

Note: "มา" at position 2 has no edges from 2→6, so it cannot form a complete path.

## N-gram Language Model

**Source:** `crates/thaime_engine/src/ngram.rs`

### Stupid Backoff

THAIME uses **Stupid Backoff** (Brants et al., 2007) for n-gram scoring. This is a simple, effective method chosen over more sophisticated approaches like Kneser-Ney for several reasons:

- No normalization required — scores are not true probabilities, which is fine for ranking
- Simple to implement and debug
- Works well with large corpora (which is what thaime-nlp produces)
- Pre-scored probabilities can be stored and loaded efficiently

### Fallback Chain

The scoring function implements a trigram→bigram→unigram fallback:

```
score(w | w₋₂, w₋₁):

    if trigram(w₋₂, w₋₁, w) exists:
        return 10^(log₁₀(P(w | w₋₂, w₋₁)))          ← direct trigram

    elif w₋₂ is available:                             ← had trigram context
        return α × bigram_score(w₋₁, w)                  but no trigram hit

    else:                                               ← no trigram context
        return bigram_score(w₋₁, w)                      (BOS or partial)


bigram_score(w₋₁, w):

    if w₋₁ is available AND bigram(w₋₁, w) exists:
        return 10^(log₁₀(P(w | w₋₁)))                ← direct bigram

    elif w₋₁ is available:                             ← had bigram context
        return α × P_unigram(w)                          but no bigram hit

    else:                                               ← BOS
        return P_unigram(w)                              no penalty at BOS
```

**Key design choice:** At beginning of sentence (BOS), where previous words are `None`, no alpha penalty is applied. The absence of context is not the same as backing off from a missed n-gram hit. This prevents BOS from unfairly penalizing all candidates.

### Alpha Penalty

The backoff factor `α` (default: 0.4) is applied each time the scorer drops to a lower-order n-gram. In the worst case (trigram context available but only unigram matches), the penalty is `α²`:

```
Trigram hit:      score ≈ P(w|w₋₂,w₋₁)
Bigram fallback:  score ≈ α × P(w|w₋₁)
Unigram fallback: score ≈ α² × P(w)
```

### Integration with Viterbi

In the Viterbi forward pass, the n-gram score is converted to a cost term:

```
ngram_bonus = ngram_weight × -ln(stupid_backoff_score)
```

Higher `ngram_weight` makes the ranking more sensitive to context. With `ngram_weight = 0`, n-gram data has no effect and scoring reverts to unigram-only.

The n-gram cost is additive: it rewards word sequences that the language model considers likely. A word that is a good unigram candidate but a poor contextual fit will be penalized relative to a contextually appropriate word.

### Binary Format

N-gram data is stored in a compact v1 binary format:

```
Offset  Size     Field
──────  ────     ─────
0       4B       Magic: "TNLM"
4       2B       Format version (u16 LE): 1
6       2B       Flags (reserved)
8       2B       Vocabulary size (u16 LE)
10      1B       Smoothing method (informational)
11      1B       Min count threshold
12      4B       Number of unigrams (u32 LE)
16      4B       Number of bigrams (u32 LE)
20      4B       Number of trigrams (u32 LE)
24      4B       Alpha (f32 LE)
28      4B       Build info (reserved)
─── Header: 32 bytes ───
32      var      String table (length-prefixed UTF-8 words)
var     var      Unigram scores (f32 LE × n_unigrams)
var     var      Bigram entries (8B each: u16 w1, u16 w2, f32 score)
var     var      Trigram entries (12B each: u16 w1, u16 w2, u16 w3, u16 pad, f32 score)
```

Scores are stored as log₁₀ probabilities. At query time, `10^score` converts back to a probability-like value.

Bigram and trigram entries are sorted by `(w1, w2)` and `(w1, w2, w3)` respectively, enabling binary search for fast lookup.

## Configuration & Tuning

**Source:** `crates/thaime_engine/src/config.rs`

All tunable parameters are centralized in `config.rs`:

### Ranking Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `LAMBDA` | 1.0 | Segmentation penalty per word. Higher values prefer fewer, longer words. Lower values allow more multi-word segmentations. |
| `MIN_FREQ` | 5×10⁻⁶ | Floor for word frequency to avoid `-ln(0)`. Effectively sets the cost ceiling for unknown/rare words. |
| `K` | 10 | Number of best candidates to track per lattice position and return to the user. |
| `NGRAM_WEIGHT` | 2.0 | N-gram cost multiplier. Higher values make ranking more context-sensitive. `0.0` disables n-gram influence. |
| `ALPHA` | 0.4 | Stupid Backoff penalty factor. Applied when falling back from trigram to bigram or bigram to unigram. |
| `BEAM_MULTIPLIER` | 4 | Global beam width = `K × BEAM_MULTIPLIER`. Controls how aggressively partial paths are pruned. |

### How Lambda Affects Results

Lambda controls the trade-off between single-word and multi-word interpretations:

```
Lambda = 0.0:  No penalty for segmentation → many short words
Lambda = 1.0:  (default) balanced
Lambda = 3.0:  Strong penalty → prefers fewer, longer words
```

Each word in a path pays the lambda penalty once. A 3-word path pays `3×LAMBDA`, while a 1-word path pays `1×LAMBDA`. This biases toward interpretations with fewer segments, which is generally desirable for Thai.

### How Ngram Weight Affects Results

```
ngram_weight = 0.0:  Pure unigram ranking (frequency only)
ngram_weight = 2.0:  (default) moderate context sensitivity
ngram_weight = 5.0:  Strong context dependence — may over-fit to common sequences
```

### How Alpha Affects Results

```
alpha = 0.1:  Harsh backoff — strongly prefers n-gram hits
alpha = 0.4:  (default) moderate backoff
alpha = 0.9:  Gentle backoff — n-gram misses are barely penalized
```

### Input Context Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `MAX_BUFFER_LEN` | 50 | Maximum input buffer length in bytes. Safety valve to prevent huge lattice construction. |
| `MAX_CONTEXT_DEPTH` | 2 | Number of previously committed words to retain for trigram context. |

### N-gram Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `TRIGRAM_MIN_COUNT` | 10 | Minimum count threshold for trigram filtering (used in TSV loading). |
| `FLOOR_PROB` | 6×10⁻⁶ | Floor probability for unseen words to avoid `log(0)`. |

## Input Context Lifecycle

**Source:** `crates/thaime_engine/src/context.rs`

### State Machine

The input context manages the lifecycle of a single input session:

```
                    push_key(ch)
              ┌─────────────────────┐
              │                     ▼
         ┌────────┐          ┌───────────┐
  init ──►  IDLE   │          │  TYPING    │
         │ buffer: │          │ buffer: "m" │
         │ ""      │ pop_key  │ candidates: │
         │         │◄─────────│ [ไม่,ไหม…] │
         └────────┘  (empty)  └──────┬─────┘
              ▲                      │
              │                      │ commit(index)
              │                      ▼
              │               ┌───────────┐
              │               │  COMMIT    │
              └───────────────│ context:   │
                  buffer=""   │ [ไม่]      │
                              └───────────┘
```

### Key Behaviors

1. **Every keystroke re-ranks**: On `push_key()` or `pop_key()`, the full ranking pipeline runs from scratch on the current buffer contents. This is simple and correct — incremental updates can be added later if profiling shows a need.

2. **Buffer length limit**: `MAX_BUFFER_LEN` (50 bytes) prevents unbounded lattice construction. Characters are rejected once the limit is reached.

3. **Commit updates context**: When a candidate is committed, each word in the candidate path is pushed onto the context history, which is then trimmed to `MAX_CONTEXT_DEPTH` (2 words) for the trigram window.

4. **Context cleared on focus change**: `clear_context()` wipes the history (e.g., when the text field loses focus or the cursor jumps). If there's active input, candidates are re-ranked without context.

5. **Case insensitive**: All input is lowercased on entry (`ch.to_ascii_lowercase()`).

6. **Non-alpha rejected**: Only ASCII alphabetic characters are accepted; digits, spaces, and symbols return `false`.

### Context Window

The committed context maintains a sliding window of the last 2 Thai words:

```
Commit "ไม่"  → context: ["ไม่"]
Commit "ใน"   → context: ["ไม่", "ใน"]
Commit "การ"  → context: ["ใน", "การ"]    ← "ไม่" dropped (MAX_CONTEXT_DEPTH=2)
```

This window provides trigram context for the next ranking pass. For multi-word candidates (e.g., "ไม่ใน"), each word is pushed individually, so the context after committing "ไม่ใน" is `["ไม่", "ใน"]`.
