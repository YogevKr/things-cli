# things-cli

Public Rust CLI for agents working against the real Things 3 app on macOS.

`things-cli` talks to `/Applications/Things3.app` through `osascript`, so it reads and writes your actual Things tasks.

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

The Homebrew flow is generator-based: use the real release URLs and checksums from `yogevkr/things-cli`.
The intended public repos are:

- app repo: `https://github.com/yogevkr/things-cli`
- tap repo: `https://github.com/yogevkr/homebrew-tap`

1. Build release artifacts locally or from the tagged GitHub Actions workflow:

```bash
make package-release
```

2. Generate a formula with real metadata:

```bash
make homebrew-formula \
  VERSION=0.2.0 \
  HOMEPAGE=https://github.com/yogevkr/things-cli \
  ARM_URL=https://github.com/yogevkr/things-cli/releases/download/v0.2.0/things-cli-v0.2.0-macos-arm64.tar.gz \
  ARM_SHA256=<arm64 sha256> \
  INTEL_URL=https://github.com/yogevkr/things-cli/releases/download/v0.2.0/things-cli-v0.2.0-macos-x86_64.tar.gz \
  INTEL_SHA256=<x86_64 sha256> \
  OUTPUT=Formula/things-cli.rb
```

3. Commit the generated `Formula/things-cli.rb` into your tap repo, then install:

```bash
brew install yogevkr/tap/things-cli
```

## Release

Tagged pushes matching `v*` build a macOS release artifact in GitHub Actions and attach:

- `things-cli-v<tag>-macos-arm64.tar.gz`
- `things-cli-v<tag>-macos-arm64.tar.gz.sha256`
- `things-cli-v<tag>-macos-x86_64.tar.gz`
- `things-cli-v<tag>-macos-x86_64.tar.gz.sha256`

Before tagging, run the local release gate:

```bash
make release-check
```

## Commands

```bash
things-cli lists
things-cli list
things-cli list --list Inbox --json
things-cli create "Review pipeline cost" --notes "check noisy logs" --tag work
things-cli get "Review pipeline cost" --json
things-cli update "Review pipeline cost" --notes "done" --tag work --tag follow-up
things-cli move "Review pipeline cost" --to Anytime
things-cli schedule "Review pipeline cost" --for tomorrow
things-cli complete "Review pipeline cost"
things-cli open "Review pipeline cost"
things-cli delete "Review pipeline cost"
```

## Agent Usage

Use `--json` for machine-readable results:

```bash
things-cli --json create "Draft weekly review" --notes-file - --list Inbox --tag agent <<'EOF'
Summarize open work, deadlines, and blocked items.
EOF

things-cli --json list --query review
things-cli --json get "Draft weekly review"
things-cli --json update "Draft weekly review" --notes "ready" --tag agent --tag weekly
things-cli --json complete "Draft weekly review"
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
