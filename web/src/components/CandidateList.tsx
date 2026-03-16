// SPDX-License-Identifier: MPL-2.0
// Candidate dropdown list — positioned near the preedit text.

import React from 'react';
import type { Candidate } from '../engine/engine-bridge';

interface CandidateListProps {
  candidates: Candidate[];
  selectedIndex: number;
  visible: boolean;
  onSelect: (index: number) => void;
}

export const CandidateList: React.FC<CandidateListProps> = ({
  candidates,
  selectedIndex,
  visible,
  onSelect,
}) => {
  if (!visible || candidates.length === 0) return null;

  return (
    <div className="candidate-list" role="listbox" aria-label="Candidate list">
      {candidates.map((c, i) => (
        <div
          key={i}
          role="option"
          aria-selected={i === selectedIndex}
          className={`candidate-item ${i === selectedIndex ? 'candidate-selected' : ''}`}
          onMouseDown={(e) => {
            e.preventDefault(); // Prevent focus loss
            onSelect(i);
          }}
        >
          <span className="candidate-number">{i + 1}</span>
          <span className="candidate-thai">{c.thai}</span>
        </div>
      ))}
    </div>
  );
};
