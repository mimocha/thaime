// SPDX-License-Identifier: MPL-2.0
// Info popover — explains what THAIME is.

import React, { useState } from 'react';

export const InfoPopover: React.FC = () => {
  const [open, setOpen] = useState(false);

  return (
    <div className="info-popover-container">
      <button
        className="info-button"
        onClick={() => setOpen((v) => !v)}
        aria-label="About THAIME"
        aria-expanded={open}
      >
        ?
      </button>

      {open && (
        <div className="info-popover" role="dialog" aria-label="About THAIME">
          <h3>What is THAIME?</h3>
          <p>
            THAIME is a Latin-to-Thai input method engine. Type Thai words using
            familiar Latin characters (romanization), and THAIME converts them
            to Thai script in real time.
          </p>
          <h4>How to use</h4>
          <ul>
            <li><strong>Type</strong> Latin characters (a–z) to compose</li>
            <li><strong>1–9</strong> to select a numbered candidate</li>
            <li><strong>Enter</strong> or <strong>Space</strong> to commit the top candidate</li>
            <li><strong>↑/↓</strong> or <strong>Tab</strong> to navigate candidates</li>
            <li><strong>Backspace</strong> to edit the input buffer</li>
            <li><strong>Escape</strong> to discard the current input</li>
          </ul>
          <p className="info-footer">
            THAIME is open source (MPL-2.0). The engine runs entirely in your
            browser via WebAssembly — no data is sent to any server.
          </p>
          <button className="info-close" onClick={() => setOpen(false)}>Close</button>
        </div>
      )}
    </div>
  );
};
