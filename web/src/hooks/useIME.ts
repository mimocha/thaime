// SPDX-License-Identifier: MPL-2.0
// IME state machine hook — manages preedit composition and candidate selection.

import { useState, useCallback, useRef, useEffect } from 'react';
import { ThaiEngine, Candidate, createEngine } from '../engine/engine-bridge';

export type IMEStatus = 'loading' | 'idle' | 'composing' | 'error';

export interface IMEState {
  status: IMEStatus;
  preedit: string;
  candidates: Candidate[];
  selectedIndex: number;
  committedText: string;
  error: string | null;
  loadProgress: number;
}

export interface UseIMEReturn extends IMEState {
  handleKeyDown: (e: React.KeyboardEvent) => void;
  commitCandidate: (index: number) => void;
  clearCommitted: () => void;
  pushKeyProgrammatic: (ch: string) => void;
  commitTop: () => void;
}

const MAX_CANDIDATES_SHOWN = 9;

export function useIME(): UseIMEReturn {
  const engineRef = useRef<ThaiEngine | null>(null);
  const [status, setStatus] = useState<IMEStatus>('loading');
  const [preedit, setPreedit] = useState('');
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [committedText, setCommittedText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loadProgress, setLoadProgress] = useState(0);

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
    const newCandidates = engine.candidates().slice(0, MAX_CANDIDATES_SHOWN);

    setPreedit(newPreedit);
    setCandidates(newCandidates);
    setSelectedIndex(0);

    if (newPreedit.length === 0) {
      setStatus('idle');
    } else {
      setStatus('composing');
    }
  }, []);

  const commitCandidate = useCallback((index: number) => {
    const engine = engineRef.current;
    if (!engine) return;

    const result = engine.commit(index);
    if (result != null) {
      setCommittedText((prev) => prev + result);
    }
    refreshState();
  }, [refreshState]);

  const pushKeyProgrammatic = useCallback((ch: string) => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.pushKey(ch.toLowerCase());
    refreshState();
  }, [refreshState]);

  const commitTop = useCallback(() => {
    commitCandidate(0);
  }, [commitCandidate]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    const engine = engineRef.current;
    if (!engine || status === 'loading' || status === 'error') return;

    const key = e.key;
    const isComposing = status === 'composing';

    // Latin character input (a-z, A-Z)
    if (key.length === 1 && /^[a-zA-Z]$/.test(key)) {
      e.preventDefault();
      engine.pushKey(key.toLowerCase());
      refreshState();
      return;
    }

    // Number keys 1-9: select candidate while composing
    if (isComposing && key.length === 1 && /^[1-9]$/.test(key)) {
      e.preventDefault();
      const idx = parseInt(key, 10) - 1;
      if (idx < candidates.length) {
        commitCandidate(idx);
      }
      return;
    }

    // Backspace while composing
    if (isComposing && key === 'Backspace') {
      e.preventDefault();
      engine.popKey();
      refreshState();
      return;
    }

    // Escape while composing: discard input
    if (isComposing && key === 'Escape') {
      e.preventDefault();
      engine.reset();
      refreshState();
      return;
    }

    // Enter while composing: commit highlighted candidate
    if (isComposing && key === 'Enter') {
      e.preventDefault();
      if (candidates.length > 0) {
        commitCandidate(selectedIndex);
      }
      return;
    }

    // Space while composing: commit top candidate
    if (isComposing && key === ' ') {
      e.preventDefault();
      if (candidates.length > 0) {
        commitCandidate(0);
      }
      return;
    }

    // Tab while composing: cycle through candidates
    if (isComposing && key === 'Tab') {
      e.preventDefault();
      if (candidates.length > 0) {
        setSelectedIndex((prev) => (prev + 1) % candidates.length);
      }
      return;
    }

    // Arrow keys while composing: navigate candidate list
    if (isComposing && key === 'ArrowDown') {
      e.preventDefault();
      if (candidates.length > 0) {
        setSelectedIndex((prev) => Math.min(prev + 1, candidates.length - 1));
      }
      return;
    }

    if (isComposing && key === 'ArrowUp') {
      e.preventDefault();
      if (candidates.length > 0) {
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
      }
      return;
    }

    // Pass-through: punctuation, numbers, space when idle
    if (!isComposing && key.length === 1) {
      e.preventDefault();
      setCommittedText((prev) => prev + key);
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
  }, [status, candidates, selectedIndex, refreshState, commitCandidate]);

  const clearCommitted = useCallback(() => {
    setCommittedText('');
  }, []);

  return {
    status,
    preedit,
    candidates,
    selectedIndex,
    committedText,
    error,
    loadProgress,
    handleKeyDown,
    commitCandidate,
    clearCommitted,
    pushKeyProgrammatic,
    commitTop,
  };
}
