# thing-cli

Public Rust CLI for agents working against the real Things 3 app on macOS.

`thing` talks to `/Applications/Things3.app` through `osascript`, so it reads and writes your actual Things tasks.

## Why

- Real backend: works on your actual Things database
- Local-first: no server required
- Agent-friendly: stable JSON output with `--json`
- Safe writes: uses the app automation layer, not direct SQLite mutation

## Install

```bash
cargo install --locked --path .
```

Or run in place:

```bash
cargo run -- --help
```

For a local binary in your user cargo bin:

```bash
make install
```

## Homebrew

The Homebrew flow is generator-based: use the real release URLs and checksums from `yogevkr/thing-cli`.
The intended public repos are:

- app repo: `https://github.com/yogevkr/thing-cli`
- tap repo: `https://github.com/yogevkr/homebrew-thing`

1. Build release artifacts locally or from the tagged GitHub Actions workflow:

```bash
make package-release
```

2. Generate a formula with real metadata:

```bash
make homebrew-formula \
  VERSION=0.1.0 \
  HOMEPAGE=https://github.com/yogevkr/thing-cli \
  ARM_URL=https://github.com/yogevkr/thing-cli/releases/download/v0.1.0/thing-v0.1.0-macos-arm64.tar.gz \
  ARM_SHA256=<arm64 sha256> \
  INTEL_URL=https://github.com/yogevkr/thing-cli/releases/download/v0.1.0/thing-v0.1.0-macos-x86_64.tar.gz \
  INTEL_SHA256=<x86_64 sha256> \
  OUTPUT=Formula/thing.rb
```

3. Commit the generated `Formula/thing.rb` into your tap repo, then install:

```bash
brew install yogevkr/thing/thing
```

## Release

Tagged pushes matching `v*` build a macOS release artifact in GitHub Actions and attach:

- `thing-<tag>-macos-arm64.tar.gz`
- `thing-<tag>-macos-arm64.tar.gz.sha256`
- `thing-<tag>-macos-x86_64.tar.gz`
- `thing-<tag>-macos-x86_64.tar.gz.sha256`

Before tagging, run the local release gate:

```bash
make release-check
```

## Commands

```bash
thing lists
thing list
thing list --list Inbox --json
thing create "Review pipeline cost" --notes "check noisy logs" --tag work
thing get "Review pipeline cost" --json
thing update "Review pipeline cost" --notes "done" --tag work --tag follow-up
thing move "Review pipeline cost" --to Anytime
thing schedule "Review pipeline cost" --for tomorrow
thing complete "Review pipeline cost"
thing open "Review pipeline cost"
thing delete "Review pipeline cost"
```

## Agent Usage

Use `--json` for machine-readable results:

```bash
thing --json create "Draft weekly review" --notes-file - --list Inbox --tag agent <<'EOF'
Summarize open work, deadlines, and blocked items.
EOF

thing --json list --query review
thing --json get "Draft weekly review"
thing --json update "Draft weekly review" --notes "ready" --tag agent --tag weekly
thing --json complete "Draft weekly review"
```

Selectors accept either a task `id` or an exact task `name`.

## Requirements

- macOS
- Things 3 installed at `/Applications/Things3.app`
- Terminal allowed to automate Things 3 when macOS prompts for permission
- `python3` available for the local e2e helper script

## Verification

```bash
make test
make e2e
```

## Exit Codes

- `0`: success
- `1`: unexpected failure
- `2`: not found
- `3`: conflict
- `4`: invalid input
