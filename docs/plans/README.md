# Plans — thaime

A plan is a design document written **before** a non-trivial change. It states the
problem, the chosen design, the decisions made along the way, and the acceptance
criteria that the implementation is reviewed against.

Research questions (ranking quality, data, romanization) belong in
[thaime-nlp](https://github.com/mimocha/thaime-nlp) and follow its research workflow.
Plans here cover engine, frontend, packaging, and web demo work; they cite thaime-nlp
summaries when the design depends on research results.

## When a Plan Is Required

Write a plan if the change does **any** of the following:

- changes a public API (Rust crate API, WASM API, C ABI);
- changes a data format or the thaime ↔ thaime-nlp data contract;
- changes user-visible input behaviour (key handling, candidates, commit rules, modes);
- adds or changes packaging, installation paths, or release process;
- needs more than one branch/PR to finish.

Anything else (bug fixes, refactors inside one module, docs, CI tweaks) needs only a
clear commit message or PR description.

A large effort gets an **umbrella plan** that lists workstreams and records the
decisions shared across them. Each workstream that meets the criteria above gets its
own child plan, linked through the `parent` field.

## File Naming

```
docs/plans/NNN-short-slug.md
```

- `NNN` is a zero-padded sequence number, assigned when the plan is created
  (next free number in the [index](#index)). Numbers are never reused.
- `short-slug` is kebab-case, a few words, e.g. `002-linux-alpha`.
- Copy [`TEMPLATE.md`](TEMPLATE.md) to start a new plan.

## Front Matter

Every plan starts with a YAML front matter block. Fields must appear in this order.

```yaml
---
id: "002"
title: Linux alpha (IBus engine, COPR/PPA packages)
status: in-progress
created: 2026-10-15
updated: 2026-10-15
authors:
  - Chawit Leosrisook
  - Claude (agent)
parent: null
milestone: linux-alpha
supersedes: []
superseded_by: null
related:
  - docs/plans/001-crowdsourcing-demo.md
  - https://github.com/mimocha/thaime-nlp/tree/main/research/008-ngram-delivery
---
```

| Field | Required | Type | Values / format |
|---|---|---|---|
| `id` | yes | quoted string | Three digits, equal to the filename prefix (`"002"`). Quoted so YAML keeps leading zeros. |
| `title` | yes | string | Human-readable title. The document's `#` heading is `Plan NNN: <title>`. |
| `status` | yes | enum | See [Status Values](#status-values). |
| `created` | yes | date | `YYYY-MM-DD`, the day the first draft was committed. Never changes. |
| `updated` | yes | date | `YYYY-MM-DD`, the day of the last change to the plan's content. |
| `authors` | yes | list of strings | People and agents who wrote the plan. Agents as `Name (agent)`. |
| `parent` | yes | quoted string or `null` | `id` of the umbrella plan, or `null` for top-level plans. |
| `milestone` | yes | string or `null` | Release milestone the plan targets, e.g. `linux-alpha`, `post-alpha`. `null` if unscheduled. |
| `supersedes` | yes | list of quoted strings | `id`s of plans this one replaces. `[]` if none. |
| `superseded_by` | yes | quoted string or `null` | `id` of the replacing plan. Required (non-null) when `status: superseded`. |
| `related` | yes | list of strings | Repo-relative paths or URLs: other plans, thaime-nlp research, issues. `[]` if none. |

All fields are always present (use `null` / `[]` for empty values), so tools and
readers never have to guess whether a field was forgotten or intentionally empty.

### Status Values

| Status | Meaning | Who sets it |
|---|---|---|
| `draft` | Being written or discussed. Open questions remain. Not approved for implementation. | Author |
| `accepted` | Approved by the maintainer. Open questions resolved or explicitly deferred. Implementation may start. | Maintainer |
| `in-progress` | Implementation has started on at least one branch. | Implementer |
| `implemented` | All acceptance criteria met and merged to `dev`. Deviations recorded in the plan. | Implementer, confirmed by maintainer |
| `rejected` | Decided against. The plan stays in the repo with the reason recorded. | Maintainer |
| `superseded` | Replaced by another plan (`superseded_by` names it). | Author of the new plan |

```
draft ──► accepted ──► in-progress ──► implemented
  │           │             │
  └───────────┴─────────────┴──► rejected | superseded
```

Plans are not rewritten after the fact. Once `accepted`, changes to the design go in
the plan's **Decision Log** (dated) instead of silently editing earlier sections; once
`implemented`, deviations go in the **Deviations** section.

## Workflow

1. **Draft.** Copy `TEMPLATE.md`, assign the next number, set `status: draft`, add the
   plan to the [index](#index). Fill in at least Objective, Context, Implementation
   Plan, and Open Questions.
2. **Decide.** The maintainer answers each open question. The answer goes under the
   question as a **Decision:** line; answers that shape the design are also promoted
   to Design Decisions with their rationale.
3. **Accept.** The maintainer sets `status: accepted`. The plan is committed to `dev`
   on its own (no implementation code in the same commit/PR).
4. **Implement.** Work happens on branches off `dev`, one workstream per branch,
   named `<type>/<plan-id>-<slug>` where `<type>` is one of `feat`, `fix`,
   `refactor`, `docs`, `ci`, `chore`, `packaging`, `spike` (e.g.
   `feat/002-ws1-engine-session`; omit `<plan-id>-` for work without a plan). `spike`
   branches hold throwaway experiments and are never merged. Set
   `status: in-progress`. Commit messages or PR descriptions name the plan.
5. **Review.** The change is checked against the plan's acceptance criteria, item by
   item. A criterion that cannot be met is a design change: record it in the
   Decision Log before merging.
6. **Close.** When every acceptance criterion is met and merged, set
   `status: implemented`, record any deviations, and update the index.

Bump `updated` whenever the plan's content changes.

### Quality Checklist (before `accepted`)

- [ ] The Context states the problem in terms of observable behaviour, not implementation.
- [ ] Non-goals are listed.
- [ ] Every design decision lists the alternatives considered and why they lost.
- [ ] Acceptance criteria are testable: each one names a test, command, or manual check.
- [ ] Every open question is resolved or explicitly deferred with an owner.
- [ ] Data-format or API changes name every affected caller/consumer.

### For Agents

- Read the plan (and its parent) before touching code for it.
- Do not decide open questions yourself; ask the maintainer and record the answer in
  the plan.
- Implement only what the plan's current revision covers. If the code disagrees with
  the plan, stop and report instead of improvising.
- When reporting completion, list each acceptance criterion with the evidence
  (test name, command output, manual check).

## Index

| # | Title | Status | Milestone | Parent |
|---|-------|--------|-----------|--------|
| 001 | [Crowdsourced typing demo](001-crowdsourcing-demo.md) | accepted | post-alpha | — |
| 002 | [Linux alpha (IBus engine, COPR/PPA packages)](002-linux-alpha.md) | accepted | linux-alpha | — |
