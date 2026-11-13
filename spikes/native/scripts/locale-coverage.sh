#!/usr/bin/env bash
# One implementation of the checks: the build tooling is also the reporter.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run --quiet --example locale_coverage
