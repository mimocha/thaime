// SPDX-License-Identifier: MPL-2.0
// Core IME text input area — handles keyboard events and renders composition state.

import React, { useRef, useEffect } from 'react';
import { PreeditDisplay } from './PreeditDisplay';
import { CandidateList } from './CandidateList';
import type { Candidate, InputMode } from '../engine/engine-bridge';
import type { IMEStatus } from '../hooks/useIME';

interface IMEInputProps {
  status: IMEStatus;
  preedit: string;
  candidates: Candidate[];
  selectedIndex: number;
  committedText: string;
  onKeyDown: (e: React.KeyboardEvent) => void;
  onCommitCandidate: (index: number) => void;
  inputMode: InputMode;
  onSwitchMode: (mode: InputMode) => void;
}

// Intentionally called Karaoke for user-friendliness, even though it's technically romanization mode
const MODE_LABELS: Record<InputMode, string> = {
  romanization: 'Karaoke',
  kedmanee: 'เกษมณี',
  latin: 'Latin',
};

const MODE_ORDER: InputMode[] = ['romanization', 'kedmanee', 'latin'];

export const IMEInput: React.FC<IMEInputProps> = ({
  status,
  preedit,
  candidates,
  selectedIndex,
  committedText,
  onKeyDown,
  onCommitCandidate,
  inputMode,
  onSwitchMode,
}) => {
  const inputRef = useRef<HTMLDivElement>(null);
  const isComposing = status === 'composing';
  const showCandidates = isComposing && inputMode === 'romanization';

  // Keep focus on the input area
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <div className="ime-input-wrapper">
      <div className="mode-selector" role="radiogroup" aria-label="Input mode">
        {MODE_ORDER.map((mode) => (
          <button
            key={mode}
            className={`mode-button${inputMode === mode ? ' active' : ''}`}
            onClick={() => {
              onSwitchMode(mode);
              inputRef.current?.focus();
            }}
            role="radio"
            aria-checked={inputMode === mode}
          >
            {MODE_LABELS[mode]}
          </button>
        ))}
        <span className="mode-shortcut-hint">Ctrl+Space</span>
      </div>

      <div
        ref={inputRef}
        className="ime-input"
        tabIndex={0}
        role="textbox"
        aria-label="Thai input area"
        onKeyDown={onKeyDown}
      >
        <span className="committed-text">{committedText}</span>
        {inputMode === 'romanization' && (
          <PreeditDisplay preedit={preedit} visible={isComposing} />
        )}
        <span className="cursor" />
      </div>

      {inputMode === 'romanization' && (
        <CandidateList
          candidates={candidates}
          selectedIndex={selectedIndex}
          visible={showCandidates}
          onSelect={onCommitCandidate}
        />
      )}
    </div>
  );
};
