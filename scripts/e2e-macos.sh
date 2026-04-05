#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="${THING_BIN:-$repo_root/target/debug/thing}"

if [[ ! -d /Applications/Things3.app ]]; then
  echo "Things 3 is required at /Applications/Things3.app" >&2
  exit 1
fi

if [[ ! -x "$bin" ]]; then
  cargo build --manifest-path "$repo_root/Cargo.toml" >/dev/null
fi

name="thing-cli-e2e-$(date +%Y%m%dT%H%M%S)"
cleanup() {
  "$bin" delete "$name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

create_json="$("$bin" --json create "$name" --notes "created by e2e" --list Inbox --tag cli --tag e2e)"
task_id="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["thing"]["id"])' <<<"$create_json")"

"$bin" --json get "$task_id" >/dev/null
"$bin" --json update "$task_id" --notes "updated by e2e" --tag cli --tag smoke >/dev/null
"$bin" --json move "$task_id" --to Anytime >/dev/null
"$bin" --json schedule "$task_id" --for tomorrow >/dev/null
"$bin" --json complete "$task_id" >/dev/null
"$bin" --json open "$task_id" >/dev/null
"$bin" --json delete "$task_id" >/dev/null

set +e
missing_output="$("$bin" get "$task_id" 2>&1)"
missing_code=$?
set -e

if [[ "$missing_code" -ne 2 ]]; then
  echo "expected exit code 2 after delete, got $missing_code" >&2
  echo "$missing_output" >&2
  exit 1
fi

echo "e2e ok: $task_id"
