// SPDX-License-Identifier: MPL-2.0
// Preedit composition rendering — shows underlined Latin input buffer.

import React from 'react';

interface PreeditDisplayProps {
  preedit: string;
  visible: boolean;
}

export const PreeditDisplay: React.FC<PreeditDisplayProps> = ({ preedit, visible }) => {
  if (!visible || !preedit) return null;

  return (
    <span className="preedit-display">
      {preedit}
    </span>
  );
};
