/*
 * SPDX-License-Identifier: MPL-2.0
 */

// Root layout: hero animation → sandbox → coming soon → footer

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { IMEInput } from './components/IMEInput';
import { InfoPopover } from './components/InfoPopover';
import { useIME } from './hooks/useIME';
import './styles/app.css';

declare const __THAIME_VERSION__: string;

type DemoPhase = 'loading' | 'ready' | 'complete';

const TAGLINE_LATIN = 'pim thai mai dai?';
const TAGLINE_THAI = 'พิมพ์คาราโอเกะแปลงเป็นไทย';
const DEMO_KEYS = 'malongchaithaime';
const TYPING_SPEED_MS = 60;
const DEMO_KEY_SPEED_MS = 80;
const DEMO_PAUSE_BEFORE_TYPE_MS = 600;
const TAGLINE_SWAP_DELAY_MS = 800;
const MIN_LOADING_DISPLAY_MS = 1500; // Minimum time to show the loading hero

/** Hook: types out a string character-by-character. */
function useTypingAnimation(text: string, speed: number, active: boolean) {
  const [displayed, setDisplayed] = useState('');
  const [done, setDone] = useState(false);

  useEffect(() => {
    if (!active) return;
    setDisplayed('');
    setDone(false);
    let i = 0;
    const interval = setInterval(() => {
      i++;
      if (i > text.length) {
        clearInterval(interval);
        setDone(true);
        return;
      }
      setDisplayed(text.slice(0, i));
    }, speed);
    return () => clearInterval(interval);
  }, [text, speed, active]);

  return { displayed, done };
}

const App: React.FC = () => {
  const ime = useIME();
  const [phase, setPhase] = useState<DemoPhase>('loading');
  const [tagline, setTagline] = useState<{ text: string; isThai: boolean }>({
    text: '',
    isThai: false,
  });
  const [showCursor, setShowCursor] = useState(true);
  const [taglineFading, setTaglineFading] = useState(false);
  const demoRanRef = useRef(false);
  const mountTimeRef = useRef(Date.now());

  // Phase 1: Type out the tagline while loading
  const isLoading = ime.status === 'loading';
  const taglineAnim = useTypingAnimation(TAGLINE_LATIN, TYPING_SPEED_MS, isLoading || phase === 'loading');

  // Update displayed tagline from typing animation during loading
  useEffect(() => {
    if (phase === 'loading') {
      setTagline({ text: taglineAnim.displayed, isThai: false });
    }
  }, [phase, taglineAnim.displayed]);

  // Track readiness gates: engine loaded + tagline animation done + minimum time elapsed
  const engineReady = ime.status === 'idle' || ime.status === 'composing';
  const [minTimeElapsed, setMinTimeElapsed] = useState(false);

  // Start minimum display timer on mount
  useEffect(() => {
    const elapsed = Date.now() - mountTimeRef.current;
    const remaining = Math.max(0, MIN_LOADING_DISPLAY_MS - elapsed);
    const t = setTimeout(() => setMinTimeElapsed(true), remaining);
    return () => clearTimeout(t);
  }, []);

  // Transition: loading → ready when ALL gates are met
  useEffect(() => {
    if (phase === 'loading' && engineReady && taglineAnim.done && minTimeElapsed) {
      setTagline({ text: TAGLINE_LATIN, isThai: false });
      // Brief pause so progress bar at 100% / "Ready" is visible
      const t = setTimeout(() => setPhase('ready'), 400);
      return () => clearTimeout(t);
    }
  }, [phase, engineReady, taglineAnim.done, minTimeElapsed]);

  // Phase 2: Scripted demo — type "malongchaithaime" into the input
  const runScriptedDemo = useCallback(() => {
    if (demoRanRef.current) return;
    demoRanRef.current = true;

    let i = 0;
    const typeNext = () => {
      if (i >= DEMO_KEYS.length) {
        // After typing completes, fade out tagline then swap
        setTimeout(() => {
          setShowCursor(false);
          setTaglineFading(true);
          // After fade-out completes, swap text and fade back in
          setTimeout(() => {
            setTagline({ text: TAGLINE_THAI, isThai: true });
            setTaglineFading(false);
            setPhase('complete');
          }, 400);
        }, TAGLINE_SWAP_DELAY_MS);
        return;
      }
      ime.pushKeyProgrammatic(DEMO_KEYS[i]);
      i++;
      setTimeout(typeNext, DEMO_KEY_SPEED_MS);
    };

    setTimeout(typeNext, DEMO_PAUSE_BEFORE_TYPE_MS);
  }, [ime.pushKeyProgrammatic]);

  useEffect(() => {
    if (phase === 'ready' && !demoRanRef.current) {
      runScriptedDemo();
    }
  }, [phase, runScriptedDemo]);

  // Error state
  if (ime.status === 'error') {
    return (
      <div className="app">
        <section className="hero hero--loading">
          <h1 className="hero-title">THA<span className="hero-title-split">I</span><span className="hero-title-me">ME</span></h1>
          <div className="error-state">
            <p>Failed to load the THAIME engine.</p>
            <p className="error-detail">{ime.error}</p>
          </div>
        </section>
      </div>
    );
  }

  const heroClass = `hero hero--${phase}`;
  const mainClass = `main-content${phase === 'loading' ? ' main-content--hidden' : ''}`;
  const footerClass = `app-footer${phase === 'loading' ? ' app-footer--hidden' : ''}`;
  const progressPct = Math.round(ime.loadProgress * 100);

  return (
    <div className="app">
      {/* ── Hero ── */}
      <section className={heroClass}>
        <h1 className="hero-title">THA<span className="hero-title-split">I</span><span className="hero-title-me">ME</span></h1>
        <p className={`hero-tagline${taglineFading ? ' hero-tagline--fading' : ''}`}>
          <span className={tagline.isThai ? 'hero-tagline-thai' : ''}>
            {tagline.text}
          </span>
          <span
            className={`hero-tagline-cursor${!showCursor ? ' hero-tagline-cursor--hidden' : ''}`}
          />
        </p>

        <div className={`progress-container${phase !== 'loading' ? ' progress-container--hidden' : ''}`}>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${progressPct}%` }} />
          </div>
          <p className="progress-label">
            {progressPct < 100 ? `Loading engine\u2026 ${progressPct}%` : 'Ready'}
          </p>
        </div>
      </section>

      {/* ── Main Content ── */}
      <main className={mainClass}>
        {/* Sandbox */}
        <section className="sandbox-section">
          <h2 className="sandbox-heading">Try it yourself <InfoPopover /></h2>
          <IMEInput
            status={ime.status}
            preedit={ime.preedit}
            committedPrefix={ime.committedPrefix}
            hybridCandidates={ime.hybridCandidates}
            selectedIndex={ime.selectedIndex}
            committedText={ime.committedText}
            onKeyDown={ime.handleKeyDown}
            onMobileInput={ime.handleMobileInput}
            onCommitCandidate={ime.commitCandidate}
            inputMode={ime.inputMode}
            onSwitchMode={ime.switchMode}
            candidatePage={ime.candidatePage}
            totalPages={ime.totalPages}
          />

          {ime.committedText && (
            <div className="output-actions">
              <button className="clear-button" onClick={ime.clearCommitted}>
                Clear
              </button>
            </div>
          )}

          {/* Keyboard shortcuts — always visible */}
          <div className="shortcuts-section">
            <p className="shortcuts-title">Keyboard Shortcuts</p>
            <div className="shortcuts-grid">
              <div className="shortcut"><kbd>Ctrl</kbd>+<kbd>Space</kbd><span>Cycle input mode</span></div>
              {ime.inputMode === 'romanization' && (
                <>
                  <div className="shortcut"><kbd>a</kbd>-<kbd>z</kbd><span>Type to compose</span></div>
                  <div className="shortcut"><kbd>1</kbd>-<kbd>6</kbd><span>Select candidate</span></div>
                  <div className="shortcut"><kbd>Enter</kbd> / <kbd>Space</kbd><span>Commit word</span></div>
                  <div className="shortcut"><kbd>↑</kbd> <kbd>↓</kbd> / <kbd>Tab</kbd><span>Navigate candidates</span></div>
                  <div className="shortcut"><kbd>Backspace</kbd><span>Edit input</span></div>
                  <div className="shortcut"><kbd>Escape</kbd><span>Discard input</span></div>
                </>
              )}
              {ime.inputMode === 'kedmanee' && (
                <>
                  <div className="shortcut"><kbd>a</kbd>-<kbd>z</kbd><span>Type Thai characters</span></div>
                  <div className="shortcut"><kbd>Shift</kbd>+key<span>Upper Thai layer</span></div>
                  <div className="shortcut"><kbd>Backspace</kbd><span>Delete character</span></div>
                </>
              )}
              {ime.inputMode === 'latin' && (
                <>
                  <div className="shortcut"><kbd>a</kbd>-<kbd>z</kbd><span>Type Latin characters</span></div>
                  <div className="shortcut"><kbd>Backspace</kbd><span>Delete character</span></div>
                </>
              )}
            </div>
          </div>
        </section>

        {/* Roadmap */}
        <section className="roadmap-section">
          <h2 className="roadmap-title">Roadmap</h2>

          <div className="roadmap-timeline">
            <div className="roadmap-item roadmap-item--active">
              <div className="roadmap-label">Now</div>
              <div className="roadmap-dot" />
              <div className="roadmap-content">
                <p>Pre-alpha development</p>
              </div>
            </div>

            <div className="roadmap-item">
              <div className="roadmap-label">Q3 2026</div>
              <div className="roadmap-dot" />
              <div className="roadmap-content">
                <p>Linux community package alpha</p>
                <ul>
                  <li>Fedora via COPR</li>
                  <li>Ubuntu via Launchpad PPA</li>
                </ul>
              </div>
            </div>

            <div className="roadmap-item">
              <div className="roadmap-label">Q4 2026</div>
              <div className="roadmap-dot" />
              <div className="roadmap-content">
                <p>Wider Linux beta</p>
                <ul>
                  <li>Fedora / RHEL via dnf</li>
                  <li>Ubuntu / Debian via apt</li>
                  <li>Arch via AUR</li>
                  <li>Other major distros</li>
                </ul>
              </div>
            </div>

            <div className="roadmap-item">
              <div className="roadmap-label">2027+</div>
              <div className="roadmap-dot" />
              <div className="roadmap-content">
                <p>Windows / macOS public release</p>
              </div>
            </div>
          </div>
        </section>

        {/* Help Improve */}
        <section className="coming-soon-section">
          <h2 className="coming-soon-title">Help improve THAIME</h2>
          <p className="coming-soon-text">
            Coming soon.
          </p>
          <a
            className="github-link"
            href="https://github.com/mimocha/thaime"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </section>
      </main>

      {/* ── Footer ── */}
      <footer className={footerClass}>
        <p>
          v{__THAIME_VERSION__} |
          MPL-2.0 |
          Powered by Rust + WebAssembly |
          Made by <a href="https://mimocha.github.io" target="_blank" rel="noopener noreferrer">mimocha</a> |
          This engine runs entirely in your browser.
        </p>
      </footer>
    </div>
  );
};

export default App;
