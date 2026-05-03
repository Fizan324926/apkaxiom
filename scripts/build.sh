#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# build.sh — `make build` equivalent, exposed as `nix run .#build` so
# external consumers can build APKAXIOM without first entering a dev
# shell. The script is intentionally thin: any logic belongs in the
# Makefile or in dedicated tools, not here.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

exec buck2 build //:all "$@"
