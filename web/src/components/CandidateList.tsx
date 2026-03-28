/*
 * SPDX-License-Identifier: MPL-2.0
 */

// Candidate dropdown list — three-zone hybrid layout:
// Zone 1: Full-sentence candidate
// Zone 2: First-word alternatives (paginated)
// Zone 3: Latin pass-through

import React from 'react';
import type { HybridCandidate } from '../hooks/useIME';

interface CandidateListProps {
  candidates: HybridCandidate[];
  selectedIndex: number;
  visible: boolean;
  onSelect: (index: number) => void;
  candidatePage: number;
  totalPages: number;
}

export const CandidateList: React.FC<CandidateListProps> = ({
  candidates,
  selectedIndex,
  visible,
  onSelect,
  candidatePage,
  totalPages,
}) => {
  if (!visible || candidates.length === 0) return null;

  // Group candidates by zone for rendering dividers
  const zones: { zone: HybridCandidate['zone']; startIdx: number; items: HybridCandidate[] }[] = [];
  let currentZone: typeof zones[number] | null = null;

  candidates.forEach((c, i) => {
    if (!currentZone || currentZone.zone !== c.zone) {
      currentZone = { zone: c.zone, startIdx: i, items: [c] };
      zones.push(currentZone);
    } else {
      currentZone.items.push(c);
    }
  });

  let itemIndex = 0;

  return (
    <div className="candidate-list" role="listbox" aria-label="Candidate list">
      {zones.map((zone, zoneIdx) => (
        <React.Fragment key={zone.zone + zoneIdx}>
          {zoneIdx > 0 && <div className="candidate-divider" />}
          {zone.items.map((c) => {
            const idx = itemIndex++;
            const isSelected = idx === selectedIndex;
            const zoneClass = `candidate-zone-${zone.zone}`;
            return (
              <div
                key={idx}
                role="option"
                aria-selected={isSelected}
                className={`candidate-item ${zoneClass} ${isSelected ? 'candidate-selected' : ''}`}
                onMouseDown={(e) => {
                  e.preventDefault();
                  onSelect(idx);
                }}
              >
                <span className="candidate-number">{idx + 1}</span>
                <span className={`candidate-thai ${zone.zone === 'pass-through' ? 'candidate-passthrough' : ''}`}>
                  {c.thai}
                </span>
                {zone.zone === 'full-sentence' && (
                  <span className="candidate-zone-label">full</span>
                )}
              </div>
            );
          })}
        </React.Fragment>
      ))}
      {totalPages > 1 && (
        <div className="candidate-page-indicator">
          {candidatePage > 0 && <span className="candidate-page-arrow">&#x25B2;</span>}
          <span className="candidate-page-num">{candidatePage + 1}/{totalPages}</span>
          {candidatePage < totalPages - 1 && <span className="candidate-page-arrow">&#x25BC;</span>}
        </div>
      )}
    </div>
  );
};
