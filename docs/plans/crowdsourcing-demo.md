# Plan: Crowdsourced Typing Demo

**Status:** Design locked (2026-09-29); implementation not started
**Target:** Q4 2026, ahead of the end-of-2026 Linux alpha
**Author:** Chawit Leosrisook (maintainer) + Claude (agent)
**Data side:** thaime-nlp `docs/change-plan-12-romanization-weights.md`

## Summary

Turn the GitHub Pages web demo into a single page that does two jobs at once:

1. **Show Thai users what an IME is** by letting them type Thai with THAIME, as the
   demo does today.
2. **Collect real typing data, only from users who opt in**, to improve THAIME's
   dictionary and ranking.

The page has two boxes: a **target** box (the Thai text to type) and an **input** box
(where the user types). Each box has its own mode toggle, giving four combinations. A
separate **contribute** toggle controls whether anything is sent to us.

## Goals

Data priorities, highest first:

1. **Romanization preferences**: how people actually spell a given Thai word in Latin.
   This is P(Latin | Thai), which the engine currently lacks entirely (see
   [Background](#background-what-the-data-improves)).
2. **Novel romanizations and missing words**: spellings our generator never produces,
   and Thai words that are not in our vocabulary.
3. **Misconversions**: cases where THAIME picked the wrong Thai word.
4. **Later, after research**: n-gram data from user-supplied text.

## Background: What the Data Improves

The engine ranks a lattice of (Latin span → Thai word) edges with Viterbi, using the
edge cost `-ln P(word) + λ + ngram_weight × -ln(context score)`:

| Term | Source today | Improved by this demo? |
|------|--------------|------------------------|
| `P(word)` | Corpus word frequency | No |
| Context score (n-gram, Stupid Backoff) | Corpus n-gram counts | Not yet (see [Data Rules](#data-rules)) |
| P(Latin \| Thai) | **Missing.** Each word's romanizations are an unweighted list of up to 100 variants, generated as a Cartesian product of syllable-component variants | **Yes, the main target** |

P(Latin | Thai) will be learned at two levels (thaime-nlp CP12):

- **Word level**: observed counts, e.g. ที่ is typed `tee` far more often than `thii`.
- **Component level**: weights on each syllable component's variants, e.g. onset ท →
  `th` / `t`. Multiplying these through a word's syllables gives an estimate for every
  word in the vocabulary, including the many that no contributor will ever type.

Contributor mode measures P(Latin | Thai) directly, because the Thai target is known and
the Latin is what the user chose to type.

## Page Design

### The 2×2 modes

| Target ↓ / Input → | THAIME input | Contributor input |
|---|---|---|
| **Generator** | Typing practice with the real IME; misconversion signal | **Cleanest romanization data** (the core data instrument) |
| **Custom** | Real-world use: missing words, misconversions | Romanizations for any word, including words not in our vocabulary |
| **Custom, empty** | Free typing (today's demo) | — |

An empty custom target with THAIME input is exactly today's demo, so nothing is lost.
First-time visitors land in that state; the other modes are one toggle away.

### Target box

**Generator mode.** A Markov generator produces semi-plausible Thai text in the style of
MonkeyType: the text only needs to feel like Thai, not make sense.

- **Runs in the engine** (Rust/WASM), sampling from the n-gram data the demo already
  downloads, so it adds no download and every word is in our vocabulary.
- **Steered toward words we need data for.** Plain sampling is dominated by
  ultra-common words (ที่, ไม่, การ). The thaime-nlp pipeline publishes a small static
  "need list" (words with the fewest distinct contributors so far), and the generator
  boosts those words.
- Text is shown **as separate tokens** (e.g. with spaces or chips between words).

**Custom mode.** The user types or pastes any Thai text.

- The page **splits it into words automatically**, so the user never has to insert
  spaces. First pass: longest match against our vocabulary. For spans we don't know:
  the browser's built-in Thai splitter (`Intl.Segmenter('th', { granularity: 'word' })`).
- The user can **tap a boundary to merge or split** tokens.
- Recorded with each submission: the raw text, the final tokens, which boundaries the
  user edited, and which tokens are not in our vocabulary (new-word candidates).
- On desktops without a Thai keyboard layout, users can paste, or type the target with
  THAIME itself.

### Input box

**THAIME mode.** The normal IME experience, including the existing hybrid candidate
list. No spaces are typed. Every commit already knows which Latin span produced which
Thai word, so the page logs `(latin_span, thai_committed, candidate_zone, rank)` per
commit. Matching against the target is only needed to show right or wrong on screen and
to flag misconversions: a character-level comparison of the Thai strings is enough.
Users can also mark a commit as a misconversion explicitly.

**Contributor mode.** Raw Latin, MonkeyType-style:

- **Space moves to the next target token**, and the current token is highlighted, so
  alignment is one-to-one by construction and needs no post-processing.
- A **skip key** marks words the user can't read or doesn't know how to spell, instead
  of producing junk.
- Nothing is converted; the user types how they would naturally spell each word,
  including spellings THAIME does not support yet.
- Known noise, like typing `sa wat dee` for one token, is filtered offline (see CP12).

### Contribute toggle and consent

- A **toggle button** shows the current state clearly (off / on).
- Turning it on opens a **pop-up** explaining what is collected, how it is used and
  published, with **"I consent"** and **"Cancel"** buttons.
- The pop-up warns: **don't type anything private in custom mode**.
- The pop-up shows the user's **contributor ID**, with a **"reset ID"** button. Sending
  us the ID is how a user requests deletion of their data.
- Turning the toggle off **discards any unsent data**.
- Before sending, users can see a **preview** of what will be sent.

## Identity

- A **random UUID**, generated in the browser and kept in local storage. It groups a
  contributor's data so one heavy contributor can't dominate the statistics, and it
  supports deletion requests.
- **No IP addresses and no device fingerprinting.** A plain hash of an IP address is
  reversible by brute force, and fingerprinting is privacy-hostile.
- Users can reset the ID at any time. Dishonest users can do the same, which is accepted;
  abuse is handled by the backend protections and offline filtering instead.

## What Is Recorded

One record per batch. Fields:

- **Submission:** schema version, app / engine / data versions, consent version,
  contributor ID, target mode (`generator` or `custom`), input mode (`thaime` or
  `contributor`), timestamp.
- **Custom target only:** raw text, final tokens, edited boundaries, out-of-vocabulary
  flags.
- **Per token, contributor mode:** target Thai, word ID (if in vocabulary), typed Latin,
  skipped flag, edited flag (backspace used), plus two engine annotations: whether the
  typed Latin is already a known key for that word, and where the engine ranks the
  target for that Latin.
- **Per commit, THAIME mode:** Latin span, Thai committed, candidate zone and rank, and
  the matching target span where known; explicit misconversion marks.

**Not recorded:** keystroke timing (close to biometric data and not needed), typing
speed (shown on screen only), IP addresses, fingerprints.

## Sending

- **Batched:** one write per batch, never per keystroke.
- **When:** automatically at the end of each generator test; via a manual
  **"Contribute now"** button; and when the page is hidden (`visibilitychange` +
  `navigator.sendBeacon`), since free and custom typing have no natural end.
- **Queued locally:** unsent batches are kept in local storage and survive reloads.
- **On any failure, keep the queue and retry later**, and show a soft message such as
  "Couldn't send. We've probably hit today's contribution limit; your data is saved and
  will be sent later." This covers both limit cases below.

## Backend

**Cloudflare Workers + D1, on the free plan.** The page stays on GitHub Pages; the
Worker is a separate endpoint the page sends batches to (with CORS allowing the Pages
origin).

- **Cost:** $0, with hard ceilings. On the free plan Cloudflare stops serving instead of
  billing: 100K Worker requests per day (Cloudflare error 1027 after that) and 100K D1
  row writes per day (queries fail after that). Both reset at midnight UTC. Firebase's
  pay-as-you-go plan was rejected because it has no hard spending cap.
- **Hitting a limit is acceptable.** The page treats it as "try again later", per
  [Sending](#sending). Note that Cloudflare's 1027 error page lacks CORS headers, so the
  browser sees it as a generic network failure rather than a readable 1027: handle all
  failures the same way.
- **Abuse protection:** Cloudflare Turnstile (free, no quota) to prove a real browser;
  the Workers rate-limiting binding; and server-side validation of every batch (schema,
  sizes, known versions) before it is stored. Offline filtering in thaime-nlp handles
  what gets through.
- **Storage:** one D1 row per batch, with the batch body as JSON. The database is
  write-only from the public's point of view: there is no read endpoint.
- **Export:** `wrangler d1 export` produces an SQL dump that the thaime-nlp pipeline
  reads.
- **Upgrade path:** if the free limits become a real constraint, the $5/month paid plan
  has no daily limit and its overage pricing is low, but it also has no hard cap. Decide
  then.

## Data Rules

- **Generator text is never used for n-gram counts.** The generator samples from our
  own n-gram data, so counting it would reinforce existing bias. Romanization data from
  generator mode is fine: it measures how people spell a *given* word, and the generator
  only affects *which* words get asked about. Enforced in the thaime-nlp pipeline via
  the `target_source` field, not by the page.
- **Custom-target text** is collected and tagged, but whether and how it feeds n-gram
  counts needs research first (users playing around, data validity).
- **Publication:** only aggregates, such as (word, spelling, distinct-contributor count),
  and only above a minimum number of distinct contributors. Raw submissions and raw
  custom text are never published. Published from thaime-nlp as versioned release
  artifacts.

## Engine and Web Work

| Area | Work |
|------|------|
| Engine (Rust/WASM) | Markov generator sampling from the loaded n-gram data, with need-list boosting |
| Engine (Rust/WASM) | Thai-side longest-match segmentation over the vocabulary, for custom targets |
| Engine (Rust/WASM) | Lookup helpers for contributor annotations: is a Latin string a key for a word, and the word's rank for that Latin |
| Engine (Rust/WASM) | Commit log with Latin spans (builds on the existing `commit_partial`) |
| Web | Target box (generator / custom with editable boundaries), input box (THAIME / contributor), contribute toggle and consent pop-up, batching and queue |
| Worker | Cloudflare Worker + D1 schema, Turnstile check, validation, rate limiting |
| Engine, later | Romanization-weight term in edge cost, once CP12 ships weighted data |

## Milestones

1. **M1, core data instrument:** generator target + contributor input + contribute toggle
   + Worker/D1 backend. This alone delivers priority 1.
2. **M2, full 2×2:** custom target with segmentation, THAIME-mode commit logging,
   misconversion marking.
3. **M3, closed loop:** first export and analysis in thaime-nlp, first published
   aggregate dataset, need list feeding back into the generator.

## Open Questions (implementation-level)

1. Generator details: bigram or trigram sampling, sentence length, how strongly to boost
   need-list words.
2. Minimum distinct contributors before a novel spelling counts, and before an aggregate
   is published.
3. License for the published dataset (e.g. CC0 or CC-BY-4.0); the consent text must
   match it.
4. Retention period for raw custom-target text after aggregation.
5. Consent text wording, in Thai and English.

## Out of Scope

- User accounts, leaderboards, and other gamification beyond on-screen typing stats
- Server-side conversion (the engine stays in the browser)
- Automatically feeding submissions into releases without maintainer review
