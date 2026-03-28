/*
 * SPDX-License-Identifier: MPL-2.0
 */

// Preedit composition rendering — shows committed prefix (fixed Thai) + underlined Latin input buffer.

import React from 'react';

interface PreeditDisplayProps {
  preedit: string;
  committedPrefix: string;
  visible: boolean;
}

export const PreeditDisplay: React.FC<PreeditDisplayProps> = ({ preedit, committedPrefix, visible }) => {
  if (!visible || (!preedit && !committedPrefix)) return null;

  return (
    <span className="preedit-container">
      {committedPrefix && (
        <span className="preedit-committed-prefix">{committedPrefix}</span>
      )}
      {preedit && (
        <span className="preedit-display">{preedit}</span>
      )}
    </span>
  );
};
