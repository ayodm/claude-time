# claude-time — handoff for next session

This file auto-loads when a Claude Code session starts in this directory.
Read it first.

---

## What this project is

A passive ROI tracker for Claude Code itself. Measures whether Claude is
*saving you time* by substituting **diff retention** for the unknowable
"time saved" baseline. A session's diff that still exists in HEAD after N
days = value; a diff that was reverted or rewritten = waste.

- **Repo:** https://github.com/ayodm/claude-time (public)
- **Stack:** Rust (single binary, no runtime deps)
- **Distribution:** crates.io + GitHub Releases + Claude Code plugin marketplace + Homebrew tap (planned)
- **License:** MIT
- **Author identity for commits:** `Ayo M <ayodm@me.com>` (the gmail address must NOT appear in commits)
- **Git config in this repo:** already set to ayodm@me.com locally; verify before committing

## What's shipped (v0.1.0)

3 commits on `main`, pushed to GitHub. Status:

- ✅ Hooks (SessionStart + SessionEnd) capture session_id, cwd, git HEAD,
  branch, transcript path, and at end: per-file diff + tool counts + tokens
  + Claude $ cost
- ✅ Retention scoring via libgit2 blame (NonGit / Rebased / NoChanges /
  Scored outcomes)
- ✅ Quadrant classification (QUICK WIN / DEEP VALUE / CHEAP WASTE /
  EXPENSIVE WASTE) + 3 non-scored outcomes
- ✅ Markdown report (`claude-time report --since 7d`)
- ✅ Storage compaction — per-session JSON for recent, rolled into
  `archive.jsonl` for older. Measured 20x disk savings (400 KB → 20 KB for
  100 sessions on APFS).
- ✅ Install/uninstall: non-destructive merge into `~/.claude/settings.json`,
  preserves user hooks, idempotent.
- ✅ 30 tests passing (26 unit + 4 integration).
- ✅ CI workflow (cargo test/clippy/fmt on macOS+Linux) + release workflow
  (tag `v*` → cross-platform binaries + crates.io publish; needs
  `CRATES_IO_TOKEN` secret on the repo).
- ✅ Plugin manifest (`.claude-plugin/plugin.json`) + marketplace catalog
  (`.claude-plugin/marketplace.json`) + hooks declaration
  (`hooks/hooks.json`) + companion skill (`skills/claude-time-report/SKILL.md`).
- ✅ Installer.sh for curl-shell installation without Rust toolchain.
- ✅ **Already installed on this machine.** `~/.cargo/bin/claude-time` is on
  PATH; `~/.claude/settings.json` has the SessionStart/SessionEnd hooks; a
  backup of pre-install settings.json sits at
  `~/.claude/settings.json.before-claude-time.bak`.

## What's queued (NOT yet implemented — start here)

Two requirements landed at the end of the previous session, both deliberate
shifts from the v0.1 plan:

### 1. Reports should be HTMX, not markdown — but also self-contained HTML

> "rather than output as a .md the outputs and reports should always be htmx"
>
> "but also readable by human if you just open up a browser"

Interpretation:
- **Default `claude-time report`** writes a self-contained `report.html` and
  opens it in the system default browser. No server needed — the HTML
  renders fine standalone.
- **`claude-time report --serve`** (or `claude-time serve`) starts a local
  `tiny_http` server, prints the URL, opens browser. HTMX attrs become live:
  click a session to expand details, change `--since` window via dropdown,
  filter by project, etc.
- **`claude-time report --stdout`** pipes HTML to stdout (CI-friendly).
- **`claude-time report --markdown`** legacy markdown for terminal use.

The HTML should be self-contained: embedded CSS, HTMX from CDN
(`https://unpkg.com/htmx.org@2`), no external assets required to render.

### 2. Reports must pinpoint where time was wasted and how

> "the reports should pin point where time was wasted and how"

Currently the report classifies whole sessions. The pinpoint requirement
means surfacing the *specific* file / iteration / pattern that caused waste,
not just "session #abc1234 is in EXPENSIVE WASTE."

Concrete signals available from existing data:

- **Files iterated** — same file edited 3+ times in one session (from
  transcript's `tool_calls` of Edit/Write/MultiEdit per file) with low
  retention (from per-file blame). High edit count × low retention =
  pinpoint waste.
- **False starts** — file written then later removed in the same session
  (detectable from transcript: Write followed by no further edits but a
  Bash `rm` or similar).
- **Cost outliers** — sessions whose Claude $ cost is > 2σ above the
  user's median session cost and whose retention is < 50%.
- **Stuck sessions** — long gaps between tool calls + many turn boundaries
  (from transcript timestamps if present).

For aggregate (across the time window):

- **Top time-wasters by file** — files with the highest cumulative
  `(edit_count × (1 - retention))` across all sessions in the window.
- **Top time-wasters by tool** — which tools' outputs got iterated on the
  most before sticking (or not).

The render must show these pinpoints prominently — the operator wants to
see *what* went wrong, not just *that* something did.

## Suggested order of implementation

Tasks were created last session but the session ended before any were
started. They're listed below in execution order — first task is the
biggest content lift; the rest cascade.

1. **Per-file waste signals (data layer)**
   - `src/transcript.rs`: extend `TranscriptStats` with
     `per_file_edits: BTreeMap<String, u32>`. Increment on every
     Edit/Write/MultiEdit/NotebookEdit tool call, keyed by `input.file_path`.
   - `src/git_history.rs`: add `score_per_file(session) -> BTreeMap<String,
     f64>` returning per-file retention rate. The existing aggregate `score`
     stays.
   - `src/model.rs`: add `WastePinpoint { file, edits, retention, severity }`
     struct + a computation that walks transcript edits + per-file retention
     to surface the top N per session.

2. **HTML+HTMX renderer (`src/html_report.rs`)** — new module, self-contained
   output. Sections: quadrant grid, **top waste pinpoints across the window
   (NEW)**, per-session table with click-to-expand row showing per-file
   pinpoints (NEW), totals, footer with caveats. Embed CSS inline. HTMX attrs
   for dynamic loads (no-op if no server).

3. **`serve` command** — add `tiny_http = "0.12"` to Cargo.toml. New module
   `src/serve.rs`. Routes: `GET /`, `GET /session/{id}` (HTMX fragment with
   per-file pinpoints), `GET /window?since=X` (HTMX fragment refreshing the
   quadrant + pinpoint list). Auto-open browser using `open` crate or
   platform-specific (`open` on macOS, `xdg-open` on Linux).

4. **Rewire CLI** — change default behavior of `report` to write
   `report.html` + open browser. Add `--stdout`, `--markdown`, `--serve`
   flags. Make sure tests pass with new defaults.

5. **Docs** — update `README.md`'s "Use" section, update
   `skills/claude-time-report/SKILL.md` to mention HTML/serve modes and
   pinpoint interpretation.

6. **Tests + ship v0.1.1** — snapshot test for HTML rendering, integration
   test that `serve` responds to a request, all existing tests still pass.
   Bump version to `0.1.1` in `Cargo.toml`, `.claude-plugin/plugin.json`,
   and `.claude-plugin/marketplace.json`. Commit, push.

## Project layout

```
.
├── Cargo.toml                              # crates.io metadata; bump version per release
├── README.md                               # 3 install routes; "Adoption" section is the tracking story
├── LICENSE                                 # MIT
├── installer.sh                            # curl-shell installer for non-Rust users
├── .claude-plugin/
│   ├── plugin.json                         # plugin manifest
│   └── marketplace.json                    # marketplace catalog (this repo IS its own marketplace)
├── hooks/
│   └── hooks.json                          # hook declarations the plugin registers
├── skills/
│   └── claude-time-report/SKILL.md         # companion skill for interpreting reports
├── src/
│   ├── main.rs                             # bin entry → cli::run
│   ├── lib.rs                              # module re-exports
│   ├── cli.rs                              # clap definitions + dispatch
│   ├── paths.rs                            # ~/.claude/claude-time/ layout, honors CLAUDE_CONFIG_DIR
│   ├── config.rs                           # TOML config (retention_window_days, hourly_rate_usd, ...)
│   ├── model.rs                            # SessionRecord + Quadrant + classify()
│   ├── hooks.rs                            # SessionStart/SessionEnd capture from stdin JSON
│   ├── transcript.rs                       # JSONL streaming parser
│   ├── git_history.rs                      # libgit2 retention scoring (NonGit/Rebased/Scored/NoChanges)
│   ├── report.rs                           # markdown renderer (will become legacy after v0.1.1)
│   ├── storage.rs                          # archive.jsonl compaction
│   └── install.rs                          # settings.json patcher
├── tests/
│   └── integration.rs                      # end-to-end via CLAUDE_CONFIG_DIR + synthetic git repos
└── .github/
    ├── workflows/
    │   ├── ci.yml                          # cargo test + clippy + fmt
    │   └── release.yml                     # tag v* → binaries + crates.io publish
    └── ISSUE_TEMPLATE/
        ├── bug_report.md
        └── feature_request.md
```

## Critical context

- **Privacy / commit identity.** Every commit must be authored as `Ayo M
  <ayodm@me.com>`. The gmail address must never appear in this repo's
  history. Local git config in this directory is already set; verify with
  `git config user.email` before committing.

- **Storage rule (from v0.1 design).** Per-session JSON files are the hot
  path; `archive.jsonl` is the cold path. Block overhead on macOS APFS is
  16x amplification — at scale, the archive matters. Don't reintroduce a
  many-small-files pattern for historical data.

- **Fail-soft hooks.** `src/hooks.rs::run_inner` errors are caught and
  logged to stderr — never propagated. A hook must NEVER crash a Claude
  Code session. Preserve this discipline when adding new capture code.

- **No telemetry.** Adoption is tracked passively via crates.io + GitHub
  download counts + stars. Do not add any HTTP egress to a third-party
  endpoint without an explicit opt-in flow.

- **Plugin marketplace install path.** Users install via
  `/plugin marketplace add ayodm/claude-time` then
  `/plugin install claude-time@claude-time`. They still need the binary on
  `$PATH` (`cargo install claude-time` or the installer.sh route). The
  plugin alone declares hooks but cannot run them without the binary.

- **Architecture constraint.** This is intentionally a CLI + hooks tool,
  not a daemon or background service. Stay sync. Don't add tokio/async
  unless `serve` truly needs it (tiny_http is sync and fine).

- **30 tests are the floor.** All must still pass after the HTMX shift.
  Add new tests for: HTML rendering snapshot, serve route responses,
  per-file pinpoint computation against a fixture session.

## Running things locally

```sh
# Test everything
cargo test

# Build + reinstall
cargo install --path .

# Inspect a real session
claude-time status
claude-time report --since 7d                  # currently markdown; will become HTML

# Try the hook flow without a real Claude session:
echo '{"session_id":"manual-test","cwd":"'$PWD'","transcript_path":"/tmp/x.jsonl","model":"claude-opus-4-7"}' \
  | claude-time hook session-start
echo '{"session_id":"manual-test","cwd":"'$PWD'"}' \
  | claude-time hook session-end
ls ~/.claude/claude-time/sessions/
```

## After v0.1.1 ships — follow-ups not started

- **Homebrew tap.** Separate repo `ayodm/homebrew-claude-time` with a
  formula that pulls the binary from a GitHub Release. Wait until v0.1.1 is
  tagged so the release URL exists.
- **`claude-time inspect <session-id>`** — pretty-print a session record
  for debugging. v0.2.
- **Optional baseline estimation.** `UserPromptSubmit` hook + 1-tap slider.
  v0.2, only if retention proves too noisy as a sole proxy.
- **zstd compression of archive.jsonl.** Tier-3 storage win (~95% smaller
  historical data). Adds the `zstd` crate. v0.2.
- **MCP server wrapper.** Expose `claude-time report` and pinpoint queries
  as MCP tools so Claude can answer "where am I wasting time?" inline.
  Speculative — only do this if it's clear it adds value beyond the SKILL.md
  approach.

## How to pick up where the last session left off

1. Read this file (you just did).
2. `cd` into this directory if you aren't already.
3. `cargo test` to confirm the 30 tests still pass.
4. Start with task 1 (per-file waste signals in `transcript.rs`,
   `git_history.rs`, `model.rs`). Run tests after each change.
5. Move through the queue in order. Commit per logical chunk with the
   `ayodm@me.com` identity.
6. When all six tasks are done, bump version to `0.1.1`, tag `v0.1.1`,
   push tag. The release workflow handles the rest.

Welcome back.
