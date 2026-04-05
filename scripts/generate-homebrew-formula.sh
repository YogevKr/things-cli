#!/usr/bin/env bash
set -euo pipefail

version=""
homepage=""
arm_url=""
arm_sha256=""
intel_url=""
intel_sha256=""
output_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="${2:?missing version}"
      shift 2
      ;;
    --homepage)
      homepage="${2:?missing homepage}"
      shift 2
      ;;
    --arm-url)
      arm_url="${2:?missing arm url}"
      shift 2
      ;;
    --arm-sha256)
      arm_sha256="${2:?missing arm sha256}"
      shift 2
      ;;
    --intel-url)
      intel_url="${2:?missing intel url}"
      shift 2
      ;;
    --intel-sha256)
      intel_sha256="${2:?missing intel sha256}"
      shift 2
      ;;
    --output)
      output_path="${2:?missing output path}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

for required in version homepage arm_url arm_sha256 intel_url intel_sha256; do
  if [[ -z "${!required}" ]]; then
    echo "missing required argument: ${required//_/-}" >&2
    exit 1
  fi
done

formula="$(cat <<EOF
class ThingsCli < Formula
  desc "things-cli: public CLI for agents working with the real Things 3 app"
  homepage "$homepage"
  version "$version"
  license "MIT"

  on_arm do
    url "$arm_url"
    sha256 "$arm_sha256"
  end

  on_intel do
    url "$intel_url"
    sha256 "$intel_sha256"
  end

  def install
    odie "things-cli requires macOS" unless OS.mac?
    bin.install "things-cli"
    prefix.install "README.md", "LICENSE"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/things-cli --version")
  end
end
EOF
)"

if [[ -n "$output_path" ]]; then
  mkdir -p "$(dirname "$output_path")"
  printf '%s\n' "$formula" > "$output_path"
else
  printf '%s\n' "$formula"
fi
