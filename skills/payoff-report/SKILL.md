---
name: payoff-report
description: Use when the user asks "did my AI session pay off?", "show my payoff report", "what's my retention rate?", or wants to interpret a payoff HTML report. Runs `payoff report` with the right flags, opens the HTML, and explains the pinpoints + drivers + quadrant in plain English. For fixing wasted time use payoff-waste-triage; for install/config issues use payoff-setup.
---

# payoff-report

Runs and **interprets** the report. Two siblings handle the rest:

- Want to *fix* wasted time, not just read it? → **payoff-waste-triage**
- Install / "not capturing" / config / hourly rate? → **payoff-setup**

## When this skill fires

- "did this session pay off?"
- "show my payoff report"
- "what's my retention rate?"
- "explain this report"

## Run the report

Default: writes self-contained HTML to `~/.claude/payoff/last-report.html`
and opens it in the browser.

```sh
payoff report --since 7d
```

Variants:

```sh
payoff report --since 30d --by project   # monthly, per-project
payoff report --serve --port 7878        # live HTMX-driven server
payoff report --stdout                   # HTML to stdout (CI / piping)
payoff report --markdown                 # legacy markdown for terminal
```

If `payoff` is not on PATH or hooks aren't installed, hand off to
**payoff-setup**.

## How to explain the report

It has four parts, in priority order:

**1. "Where time was wasted"** (leads the report) — the answer to the most
common question. Walk the top 3 pinpoints:

- **SEVERE** — 5+ edits, <10% retention. "Look here first."
- **ITERATED** — 3+ edits, <50% retention. Churn that didn't stick.
- **LOST** — single edit, full loss. Written, then reverted by hand.

Each pinpoint has a plain-English "Why" column — read it to the user. If they
want to *act* on these rather than just understand them, switch to
**payoff-waste-triage**.

**2. Drivers** — groups sessions by environment feature (active skills,
CLAUDE.md hash, model, edit pattern) with retention/cost deltas vs the
all-sessions baseline. Answers "did changing X help?" — always flag it as
correlation, not causation.

**3. Quadrant** — whole-session summary:

| Quadrant | Meaning |
|---|---|
| **QUICK WIN** | Short session, diff still in HEAD. Cheap value. |
| **DEEP VALUE** | Long session, diff still in HEAD. Earned its cost. |
| **CHEAP WASTE** | Short session, diff reverted. Cheap but unproductive. |
| **EXPENSIVE WASTE** | Long session, diff gone. The signal worth examining. |

Plus three non-scored outcomes: **PENDING** (window not yet elapsed),
**REBASED** (commit squashed/rebased away), **UNMEASURABLE** (ran outside git).

**4. Totals** — token spend, dollar cost, cache hit ratio, sessions by model.

## Server mode (`--serve`)

If the user wants to *explore* rather than read:

```sh
payoff report --serve
```

HTMX page: click a session row to expand per-file pinpoints, tool-call mix,
tokens, cwd; click a driver row to drill into that group's sessions.

## What the report does NOT measure

Remind the user (the footer says it too):

- No absolute "time saved" claim (no baseline)
- No code quality beyond retention
- No subjective satisfaction or learning value
- Drivers are correlation, not causation — pin a CLAUDE.md hash and toggle
  one feature to compare cleanly
