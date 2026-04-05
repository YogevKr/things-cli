#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version=""
arch_override=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="${2:?missing version}"
      shift 2
      ;;
    --arch)
      arch_override="${2:?missing arch}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$version" ]]; then
  version="$(python3 - <<'PY' "$repo_root/Cargo.toml"
import pathlib
import re
import sys

text = pathlib.Path(sys.argv[1]).read_text()
match = re.search(r'^version = "([^"]+)"$', text, re.MULTILINE)
if not match:
    raise SystemExit("failed to read version from Cargo.toml")
print(match.group(1))
PY
)"
fi

arch="${arch_override:-$(uname -m)}"
dist_dir="$repo_root/dist"
package_dir="$dist_dir/thing-${version}-macos-${arch}"
archive_path="${package_dir}.tar.gz"
sha_path="${archive_path}.sha256"

mkdir -p "$dist_dir"
rm -rf "$package_dir" "$archive_path" "$sha_path"
mkdir -p "$package_dir"

if [[ ! -x "$repo_root/target/release/thing" ]]; then
  cargo build --manifest-path "$repo_root/Cargo.toml" --locked --release >/dev/null
fi

cp "$repo_root/target/release/thing" "$package_dir/"
cp "$repo_root/README.md" "$repo_root/LICENSE" "$package_dir/"
tar -C "$dist_dir" -czf "$archive_path" "$(basename "$package_dir")"
shasum -a 256 "$archive_path" > "$sha_path"

echo "$archive_path"
