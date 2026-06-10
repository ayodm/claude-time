---
name: payoff-waste-triage
description: Use when the user wants to REDUCE wasted time, not just read a report — "where am I wasting time and how do I fix it?", "why does Claude keep churning this file?", "help me stop the expensive-waste sessions", "diagnose my low retention". Goes beyond explaining the report: reads the wasted-file transcripts, finds the prompts that produced reverted diffs, and proposes concrete prompt / CLAUDE.md fixes.
---

# payoff-waste-triage

This is the *actionable* counterpart to `payoff-report`. The report tells the
user **where** time was wasted; this skill figures out **why** and proposes a
**fix**. It is agentic — it reads transcripts and edits-history, then
recommends changes the user can make.

## When this skill fires

- "where am I wasting time, and how do I fix it?"
- "why does Claude keep rewriting this file?"
- "help me cut the expensive-waste sessions"
- "my retention is low — what's the actual problem?"
- "triage my worst session"

If the user only wants to *read* a report, use `payoff-report` instead.

## The triage loop

### 1. Get the pinpoints

```sh
payoff report --since 30d --stdout
```

Use a wide window (`30d`) — waste patterns rhyme across sessions, and you need
several to see the rhyme. The pinpoint section ranks files by **waste score**.
Tiers, worst first:

- **SEVERE** — 5+ edits on one file, <10% retention. Start here.
- **ITERATED** — 3+ edits, <50% retention. Visible churn that didn't stick.
- **LOST** — single edit, full retention loss. Written, then reverted by hand.

Pull the top 3 pinpoints and the **session IDs** attached to them.

### 2. Read the prompts behind the waste

For each high-waste session, open its transcript:

```
~/.claude/projects/<project>/<session-id>.jsonl
```

(Find `<project>` by the cwd shown in the session record at
`~/.claude/payoff/sessions/<session-id>.json`.)

Read the `role: user` messages that preceded the edits to the wasted file.
You're looking for the *shape* of the request that produced churn:

- Vague / underspecified asks → Claude guesses → user reverts.
- Missing constraints (file already had a convention Claude didn't know).
- A task repeated across sessions that Claude never quite nails.
- Big-bang asks that would have survived as smaller, verified steps.

### 3. Find the rhyme

Across the top sessions, the waste almost always shares a root cause. Name it
explicitly: *"every time you ask for X without specifying Y, the diff gets
reverted."* One root cause usually explains several pinpoints.

### 4. Propose the fix

Tie the recommendation to the root cause. Common fixes:

- **Tighten the prompt** — add the missing constraint/convention up front.
  Show the user a rewritten version of one of their actual prompts.
- **Add a CLAUDE.md rule** — if the convention Claude kept missing is
  project-wide, encode it once in the repo's CLAUDE.md. (The Drivers section
  will then show whether the new hash moved retention.)
- **Change the workflow** — if SEVERE files are exploratory scratch, that's
  legitimate iteration, not waste; suggest excluding the dir via
  `[exclude] paths` in `config.toml` rather than "fixing" it.

### 5. Close the loop

Tell the user how to verify the fix worked: keep working normally, then in a
week re-run `payoff report --by project` and compare the driver group for the
changed CLAUDE.md hash against baseline. Flag honestly: this is correlation,
not proof — pin one variable and change one thing at a time.

## Distinguishing real waste from healthy iteration

Not all low retention is a problem. Before recommending a fix, rule out:

- **REBASED** sessions — high count means an aggressive squash/rebase
  workflow is erasing the signal, not Claude doing bad work.
- **Exploratory work** — prototyping legitimately throws code away.
- **PENDING** — the retention window hasn't elapsed; not yet judgeable.

Only `EXPENSIVE WASTE` (long session, diff gone) and repeated SEVERE/ITERATED
pinpoints on *non-scratch* files are worth a prompt/CLAUDE.md intervention.

## What this can't see

Retention is a proxy. A reverted diff might have been a valuable dead-end that
taught the user something. Don't pathologize every revert — focus on the
*repeated* patterns, which are the ones a prompt or rule can actually fix.
