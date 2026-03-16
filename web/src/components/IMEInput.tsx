// SPDX-License-Identifier: MPL-2.0
// Core IME text input area — handles keyboard events and renders composition state.

import React, { useRef, useEffect } from 'react';
import { PreeditDisplay } from './PreeditDisplay';
import { CandidateList } from './CandidateList';
import type { Candidate } from '../engine/engine-bridge';
import type { IMEStatus } from '../hooks/useIME';

interface IMEInputProps {
  status: IMEStatus;
  preedit: string;
  candidates: Candidate[];
  selectedIndex: number;
  committedText: string;
  onKeyDown: (e: React.KeyboardEvent) => void;
  onCommitCandidate: (index: number) => void;
}

export const IMEInput: React.FC<IMEInputProps> = ({
  status,
  preedit,
  candidates,
  selectedIndex,
  committedText,
  onKeyDown,
  onCommitCandidate,
}) => {
  const inputRef = useRef<HTMLDivElement>(null);
  const isComposing = status === 'composing';

  // Keep focus on the input area
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <div className="ime-input-wrapper">
      <div
        ref={inputRef}
        className="ime-input"
        tabIndex={0}
        role="textbox"
        aria-label="Thai input area"
        aria-describedby="ime-instructions"
        onKeyDown={onKeyDown}
      >
        <span className="committed-text">{committedText}</span>
        <PreeditDisplay preedit={preedit} visible={isComposing} />
        <span className="cursor" />
      </div>

      <CandidateList
        candidates={candidates}
        selectedIndex={selectedIndex}
        visible={isComposing}
        onSelect={onCommitCandidate}
      />
    </div>
  );
};
