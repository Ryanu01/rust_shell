#!/usr/bin/env bash
set -euo pipefail

for cmd in rustc cargo; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "Error: $cmd is not installed" >&2
    exit 1
  fi
done

release=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help | -h)
      echo "Usage: $0 [--release] [-- <args>]"
      echo
      echo "Options:"
      echo "  --release    Build and run in release mode"
      echo "  --           Pass remaining arguments to rush"
      echo "  --help, -h   Show this help message"
      exit 0
      ;;
    --release)
      release=true
      shift
      ;;
    --)
      shift
      break
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--release] [-- <args>]" >&2
      exit 1
      ;;
  esac
done

cmd=(cargo run)
$release && cmd+=(--release)
[[ $# -gt 0 ]] && cmd+=(-- "$@")

exec "${cmd[@]}"
