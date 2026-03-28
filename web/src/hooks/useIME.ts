/*
 * SPDX-License-Identifier: MPL-2.0
 */

// IME state machine hook — manages preedit composition and candidate selection.
// Supports hybrid candidate UX: full-sentence + first-word alternatives + pass-through.

import { useState, useCallback, useRef, useEffect } from 'react';
import { ThaiEngine, Candidate, FirstWordCandidate, InputMode, createEngine } from '../engine/engine-bridge';

export type IMEStatus = 'loading' | 'idle' | 'composing' | 'error';

/** A visible candidate item in the hybrid candidate list. */
export interface HybridCandidate {
  /** Display text (Thai or Latin pass-through). */
  thai: string;
  /** Candidate zone: full-sentence, first-word, or pass-through. */
  zone: 'full-sentence' | 'first-word' | 'pass-through';
  /** For first-word candidates: bytes to consume from input buffer. */
  endPos?: number;
  /** Original index into the engine's full candidate list (for full-sentence only). */
  engineIndex?: number;
}

export interface IMEState {
  status: IMEStatus;
  preedit: string;
  candidates: Candidate[];
  hybridCandidates: HybridCandidate[];
  selectedIndex: number;
  committedText: string;
  committedPrefix: string;
  error: string | null;
  loadProgress: number;
  candidatePage: number;
  totalPages: number;
}

export interface UseIMEReturn extends IMEState {
  handleKeyDown: (e: React.KeyboardEvent) => void;
  handleMobileInput: (e: React.FormEvent<HTMLInputElement>) => void;
  commitCandidate: (index: number) => void;
  clearCommitted: () => void;
  pushKeyProgrammatic: (ch: string) => void;
  commitTop: () => void;
  inputMode: InputMode;
  switchMode: (mode: InputMode) => void;
}

const FIRST_WORD_PAGE_SIZE = 4;
const FIRST_WORD_SUBSEQUENT_PAGE_SIZE = 6;
const MAX_FIRST_WORD_TOTAL = 30;

/**
 * Build the visible hybrid candidate list for a given page.
 *
 * Page 0: 1 full-sentence + up to 4 first-word alts + 1 pass-through = max 6
 * Page 1+: up to 6 first-word alts per page
 */
function buildHybridCandidates(
  fullSentence: Candidate | null,
  firstWords: FirstWordCandidate[],
  preedit: string,
  page: number,
): { items: HybridCandidate[]; totalPages: number } {
  // Limit first-word candidates
  const cappedFirstWords = firstWords.slice(0, MAX_FIRST_WORD_TOTAL);

  // Deduplicate: if the top first-word is identical to the full-sentence (single-word input), exclude it
  const dedupedFirstWords = fullSentence
    ? cappedFirstWords.filter((fw) => fw.thai !== fullSentence.thai)
    : cappedFirstWords;

  // Calculate total pages
  const firstWordCount = dedupedFirstWords.length;
  let totalPages = 1;
  if (firstWordCount > FIRST_WORD_PAGE_SIZE) {
    totalPages = 1 + Math.ceil((firstWordCount - FIRST_WORD_PAGE_SIZE) / FIRST_WORD_SUBSEQUENT_PAGE_SIZE);
  }

  const items: HybridCandidate[] = [];

  if (page === 0) {
    // Zone 1: Full-sentence
    if (fullSentence) {
      items.push({
        thai: fullSentence.thai,
        zone: 'full-sentence',
        engineIndex: 0,
      });
    }

    // Zone 2: First-word alternatives (up to FIRST_WORD_PAGE_SIZE)
    for (const fw of dedupedFirstWords.slice(0, FIRST_WORD_PAGE_SIZE)) {
      items.push({
        thai: fw.thai,
        zone: 'first-word',
        endPos: fw.endPos,
      });
    }

    // Zone 3: Pass-through
    if (preedit) {
      items.push({
        thai: preedit,
        zone: 'pass-through',
      });
    }
  } else {
    // Subsequent pages: only first-word alternatives
    const offset = FIRST_WORD_PAGE_SIZE + (page - 1) * FIRST_WORD_SUBSEQUENT_PAGE_SIZE;
    const pageItems = dedupedFirstWords.slice(offset, offset + FIRST_WORD_SUBSEQUENT_PAGE_SIZE);
    for (const fw of pageItems) {
      items.push({
        thai: fw.thai,
        zone: 'first-word',
        endPos: fw.endPos,
      });
    }
  }

  return { items, totalPages };
}

export function useIME(): UseIMEReturn {
  const engineRef = useRef<ThaiEngine | null>(null);
  const [status, setStatus] = useState<IMEStatus>('loading');
  const [preedit, setPreedit] = useState('');
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [hybridCandidates, setHybridCandidates] = useState<HybridCandidate[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [committedText, setCommittedText] = useState('');
  const [committedPrefix, setCommittedPrefix] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loadProgress, setLoadProgress] = useState(0);
  const [inputMode, setInputMode] = useState<InputMode>('romanization');
  const [candidatePage, setCandidatePage] = useState(0);
  const [totalPages, setTotalPages] = useState(1);

  // Initialize engine on mount
  useEffect(() => {
    let cancelled = false;
    createEngine((loaded, total) => {
      if (!cancelled) {
        setLoadProgress(Math.min(loaded / total, 1));
      }
    })
      .then((engine) => {
        if (cancelled) return;
        engineRef.current = engine;
        setLoadProgress(1);
        setStatus('idle');
      })
      .catch((err) => {
        if (cancelled) return;
        setError(String(err));
        setStatus('error');
      });
    return () => { cancelled = true; };
  }, []);

  const refreshState = useCallback(() => {
    const engine = engineRef.current;
    if (!engine) return;

    const newPreedit = engine.preedit();
    const newCandidates = engine.candidates();
    const newFirstWords = engine.firstWordCandidates();

    setPreedit(newPreedit);
    setCandidates(newCandidates);
    setCandidatePage(0);
    setSelectedIndex(0);

    // Build hybrid candidate list for page 0
    const fullSentence = newCandidates.length > 0 ? newCandidates[0] : null;
    const { items, totalPages: tp } = buildHybridCandidates(fullSentence, newFirstWords, newPreedit, 0);
    setHybridCandidates(items);
    setTotalPages(tp);

    if (newPreedit.length === 0) {
      setStatus('idle');
    } else {
      setStatus('composing');
    }
  }, []);

  /** Navigate to a specific candidate page. */
  const goToPage = useCallback((page: number) => {
    const engine = engineRef.current;
    if (!engine) return;

    const currentPreedit = engine.preedit();
    const currentCandidates = engine.candidates();
    const currentFirstWords = engine.firstWordCandidates();

    const fullSentence = currentCandidates.length > 0 ? currentCandidates[0] : null;
    const { items, totalPages: tp } = buildHybridCandidates(fullSentence, currentFirstWords, currentPreedit, page);

    setCandidatePage(page);
    setHybridCandidates(items);
    setTotalPages(tp);
    setSelectedIndex(0);
  }, []);

  const commitHybridCandidate = useCallback((index: number) => {
    const engine = engineRef.current;
    if (!engine) return;
    if (index < 0 || index >= hybridCandidates.length) return;

    const candidate = hybridCandidates[index];

    if (candidate.zone === 'full-sentence') {
      // Commit entire sentence — existing behavior
      const result = engine.commit(0);
      if (result != null) {
        setCommittedText((prev) => prev + committedPrefix + result);
        setCommittedPrefix('');
      }
    } else if (candidate.zone === 'first-word' && candidate.endPos != null) {
      // Partial commit — commit just this word, keep remainder
      const success = engine.commitPartial(candidate.thai, candidate.endPos);
      if (success) {
        setCommittedPrefix((prev) => prev + candidate.thai);
      }
    } else if (candidate.zone === 'pass-through') {
      // Commit raw Latin text as-is
      const raw = engine.preedit();
      engine.reset();
      setCommittedText((prev) => prev + committedPrefix + raw);
      setCommittedPrefix('');
    }

    refreshState();
  }, [hybridCandidates, committedPrefix, refreshState]);

  // Legacy commitCandidate for the scripted demo — commits full-sentence at index 0
  const commitCandidate = useCallback((index: number) => {
    const engine = engineRef.current;
    if (!engine) return;

    const result = engine.commit(index);
    if (result != null) {
      setCommittedText((prev) => prev + committedPrefix + result);
      setCommittedPrefix('');
    }
    refreshState();
  }, [committedPrefix, refreshState]);

  const pushKeyProgrammatic = useCallback((ch: string) => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.pushKey(ch.toLowerCase());
    refreshState();
  }, [refreshState]);

  const commitTop = useCallback(() => {
    commitCandidate(0);
  }, [commitCandidate]);

  const switchMode = useCallback((mode: InputMode) => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.setMode(mode);
    setInputMode(mode);
    setCommittedPrefix('');
    refreshState();
  }, [refreshState]);

  const handleCharInput = useCallback((ch: string) => {
    const engine = engineRef.current;
    if (!engine || status === 'loading' || status === 'error') return;

    if (inputMode === 'kedmanee') {
      if (ch === ' ') setCommittedText((prev) => prev + ' ');
      else {
        const result = engine.processKey(ch);
        if (result != null && result.length > 0) setCommittedText((prev) => prev + result);
      }
      return;
    }
    if (inputMode === 'latin') {
      setCommittedText((prev) => prev + ch);
      return;
    }
    // Romanization
    if (/^[a-zA-Z]$/.test(ch)) { engine.pushKey(ch.toLowerCase()); refreshState(); return; }
    if (status === 'composing' && /^[1-6]$/.test(ch)) {
      const idx = parseInt(ch, 10) - 1;
      if (idx < hybridCandidates.length) commitHybridCandidate(idx);
      return;
    }
    if (status !== 'composing') setCommittedText((prev) => prev + ch);
  }, [status, hybridCandidates, inputMode, refreshState, commitHybridCandidate]);

  const handleMobileInput = useCallback((e: React.FormEvent<HTMLInputElement>) => {
    const data = (e.nativeEvent as InputEvent).data;
    if (data) handleCharInput(data);
    (e.target as HTMLInputElement).value = '';
  }, [handleCharInput]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    const engine = engineRef.current;
    if (!engine || status === 'loading' || status === 'error') return;

    const key = e.key;

    // Ctrl+Space: cycle input modes
    if (key === ' ' && e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
      e.preventDefault();
      const modes: InputMode[] = ['romanization', 'kedmanee', 'latin'];
      const nextIdx = (modes.indexOf(inputMode) + 1) % modes.length;
      switchMode(modes[nextIdx]);
      return;
    }

    // ── Kedmanee mode: direct key→Thai mapping ──────────────────
    if (inputMode === 'kedmanee') {
      if (key === 'Backspace') {
        e.preventDefault();
        setCommittedText((prev) => {
          const arr = [...prev];
          arr.pop();
          return arr.join('');
        });
        return;
      }
      if (key.length === 1) {
        e.preventDefault();
        handleCharInput(key);
        return;
      }
      return;
    }

    // ── Latin mode: pass-through ────────────────────────────────
    if (inputMode === 'latin') {
      if (key === 'Backspace') {
        e.preventDefault();
        setCommittedText((prev) => {
          const arr = [...prev];
          arr.pop();
          return arr.join('');
        });
        return;
      }
      if (key.length === 1) {
        e.preventDefault();
        handleCharInput(key);
        return;
      }
      return;
    }

    // ── Romanization mode ───────────────────────────────────────
    const isComposing = status === 'composing';

    // Latin character input (a-z, A-Z)
    if (key.length === 1 && /^[a-zA-Z]$/.test(key)) {
      e.preventDefault();
      handleCharInput(key);
      return;
    }

    // Number keys 1-6: select candidate while composing
    if (isComposing && key.length === 1 && /^[1-6]$/.test(key)) {
      e.preventDefault();
      handleCharInput(key);
      return;
    }

    // Backspace while composing
    if (isComposing && key === 'Backspace') {
      e.preventDefault();
      engine.popKey();
      refreshState();
      return;
    }

    // Escape while composing: discard input and committed prefix
    if (isComposing && key === 'Escape') {
      e.preventDefault();
      engine.reset();
      setCommittedPrefix('');
      refreshState();
      return;
    }

    // Enter while composing: commit highlighted candidate
    if (isComposing && key === 'Enter') {
      e.preventDefault();
      if (hybridCandidates.length > 0) {
        commitHybridCandidate(selectedIndex);
      }
      return;
    }

    // Space while composing: commit full-sentence (#1)
    if (isComposing && key === ' ') {
      e.preventDefault();
      if (hybridCandidates.length > 0) {
        // Find the full-sentence candidate (always index 0 on page 0)
        if (candidatePage === 0 && hybridCandidates[0]?.zone === 'full-sentence') {
          commitHybridCandidate(0);
        } else {
          // If on a later page, go back to page 0 and commit full-sentence
          // For simplicity, commit via engine directly
          const result = engine.commit(0);
          if (result != null) {
            setCommittedText((prev) => prev + committedPrefix + result);
            setCommittedPrefix('');
          }
          refreshState();
        }
      }
      return;
    }

    // Tab while composing: cycle through candidates
    if (isComposing && key === 'Tab') {
      e.preventDefault();
      if (hybridCandidates.length > 0) {
        const nextIdx = (selectedIndex + 1) % hybridCandidates.length;
        setSelectedIndex(nextIdx);
      }
      return;
    }

    // Arrow keys: navigate candidate list with pagination
    if (isComposing && key === 'ArrowDown') {
      e.preventDefault();
      if (hybridCandidates.length > 0) {
        if (selectedIndex < hybridCandidates.length - 1) {
          setSelectedIndex((prev) => prev + 1);
        } else if (candidatePage < totalPages - 1) {
          goToPage(candidatePage + 1);
        }
      }
      return;
    }

    if (isComposing && key === 'ArrowUp') {
      e.preventDefault();
      if (hybridCandidates.length > 0) {
        if (selectedIndex > 0) {
          setSelectedIndex((prev) => prev - 1);
        } else if (candidatePage > 0) {
          // Go to previous page, select last item
          goToPage(candidatePage - 1);
          // We need to set selectedIndex after the page rebuilds
          // goToPage sets it to 0, so we override it. However, we
          // don't know the new list length yet. Use a ref workaround:
          // For simplicity, just go to previous page at index 0.
        }
      }
      return;
    }

    // Pass-through: punctuation, numbers, space when idle
    if (!isComposing && key.length === 1) {
      e.preventDefault();
      handleCharInput(key);
      return;
    }

    // Backspace when idle: remove last committed character
    if (!isComposing && key === 'Backspace') {
      e.preventDefault();
      setCommittedText((prev) => {
        // Handle surrogate pairs / grapheme clusters
        const arr = [...prev];
        arr.pop();
        return arr.join('');
      });
      return;
    }
  }, [status, hybridCandidates, selectedIndex, candidatePage, totalPages, committedPrefix, inputMode, refreshState, commitHybridCandidate, switchMode, handleCharInput, goToPage]);

  const clearCommitted = useCallback(() => {
    setCommittedText('');
    setCommittedPrefix('');
  }, []);

  return {
    status,
    preedit,
    candidates,
    hybridCandidates,
    selectedIndex,
    committedText,
    committedPrefix,
    error,
    loadProgress,
    candidatePage,
    totalPages,
    handleKeyDown,
    handleMobileInput,
    commitCandidate,
    clearCommitted,
    pushKeyProgrammatic,
    commitTop,
    inputMode,
    switchMode,
  };
}
