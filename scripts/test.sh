#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# test.sh — `make test` equivalent. Exposed as `nix run .#test`.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

exec buck2 test \
  //crates/axiom-l0:axiom-l0-test \
  //crates/axiom-l1-rs:axiom-l1-rs-test \
  //crates/axiom-ir:axiom-ir-test \
  "$@"
