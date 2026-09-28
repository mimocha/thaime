# Concept: Crowdsourced Typing Demo

**Status:** Concept (not yet planned in detail)
**Date:** 2026-09-28
**Target:** Q4 2026, ahead of the Fedora COPR alpha
**Author:** Chawit Leosrisook (maintainer) + Claude (agent)

## Summary

A typing-practice web app built on the existing web demo, in the style of
[monkeytype](https://monkeytype.com/). The app shows a passage of Thai text; the user
types it in Latin romanization and THAIME converts it as they go. Users can voluntarily
share their typing data with the project:

- **Misconversions:** the user marks a word THAIME converted wrongly and submits it.
- **Preferred romanizations:** how real users actually spell each Thai word in Latin,
  used to improve the romanization probabilities in the trie.

The goal is a source of real-user data, which the NLP pipelines currently lack.
Dictionary coverage, variant weights, and ranking are all derived from corpora and
rule-based generation, with no signal from how people actually type (see thaime-nlp
change plans 09 and 10, "Limitations").

## Why a typing test

In free-form typing we never know what the user meant to write. In a typing test the
target Thai text is known in advance, so every completed passage is a labelled example:

| We know | We observe |
|---------|------------|
| Target Thai word(s) | The Latin the user typed for them |
| | Where THAIME ranked the target among its candidates |
| | Which candidate the user committed |

This gives three kinds of data with no extra effort from the user:

1. **Romanization pairs** `(thai_word, latin_typed)`, which aggregate into
   per-word preferred-romanization frequencies for the trie.
2. **Automatic misconversion candidates**: the committed Thai differs from the target,
   or the target ranked below #1.
3. **Coverage gaps**: the target never appeared in the candidate list at all (an
   unknown variant, or a word missing from the dictionary).

Explicit "mark as misconversion" reports then act as a high-confidence confirmation
layer on top of the automatic signal, and help separate user typos from engine errors.

## Components

- **Frontend:** extends `web/`. The existing demo already has a placeholder
  "Help improve THAIME — Coming soon" section to link from. The engine keeps running
  client-side via WASM; only submitted results leave the browser.
- **Prompt text:** Thai passages to type. They need a license that allows display and
  reuse, and should cover common words first, then loanwords and names once
  nlp-data v1.1.0 ships.
- **Collection backend:** a small server-side service that accepts submissions and
  stores them for analysis. This is the one paid component.
- **Analysis → pipeline:** a thaime-nlp pipeline stage that turns exported submissions
  into romanization-frequency and override data for the trie build.

## Open Questions

1. **Backend service.** Which hosted service to use: a serverless function plus a small
   database is likely enough at the expected volume. Selection criteria: cost at low
   volume, data export, and data residency.
2. **Consent and privacy.** Opt-in only, with a clear notice of what is collected. Avoid
   collecting anything personal: no accounts, no IP storage, a random per-session ID at
   most. Check Thailand's PDPA obligations for anonymous data.
3. **What counts as a submission.** Per word, per passage, or per session? Should raw
   keystroke sequences (including backspaces) be kept, or only the final Latin per word?
4. **Aligning Latin to Thai words.** Multi-word passages need the typed Latin split per
   target word. The engine's lattice path gives spans for its own output, but not
   necessarily for the target when THAIME got the segmentation wrong.
5. **Abuse and noise.** Rate limiting and basic sanity filters (such as dropping
   submissions far below a typing-accuracy threshold) so junk does not reach the
   pipeline.
6. **Prompt source licensing.** Which corpus the passages come from, and whether its
   license permits showing it in the app.
7. **Data license.** The license for the collected dataset, and whether it will be
   published.

## Out of Scope (for the first version)

- User accounts, leaderboards, and other gamification
- Server-side conversion (the engine stays client-side)
- Automatically feeding submissions into releases without maintainer review
