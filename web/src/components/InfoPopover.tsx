/*
 * SPDX-License-Identifier: MPL-2.0
 */

// Info popover — explains what THAIME is.

import React, { useState, useEffect, useRef } from 'react';

export const InfoPopover: React.FC = () => {
  const [open, setOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  return (
    <div className="info-popover-container" ref={popoverRef}>
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
          <p className="info-tagline">pim thai mai dai?</p>
          <p>
            <strong>THAIME</strong> <em>(Thai Input Method Editor)</em> lets you type phonetically in Latin and convert into Thai script.
          </p>
          <p className="info-thai">
            สำหรับคนไม่ชอบใช้<a href='https://www.keychron.co.th/blogs/article/kedmanee-pattachote'>แป้นพิมพ์ไทยเกษมณี</a>
            THAIME เป็นซอฟต์แวร์ที่ช่วยให้คุณสามารถพิมพ์ภาษาไทยแบบคาราโอเกะแล้วแปลงเป็นตัวหนังสือไทยได้
          </p>
          <p>
            <span className="info-example">"sawasdee"</span> → <span className="info-example-thai">"สวัสดี"</span>
          </p>
        </div>
      )}
    </div>
  );
};
