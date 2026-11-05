#!/usr/bin/env bash
# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD
#
# Scaffold a new trycmd snapshot test, removing the per-test ceremony of
# hand-creating the `.in`/`.out` sandbox directories, the `.keep` markers
# that keep empty ones in git, and the shared `some-identity` fixture every
# store-touching test needs (`identity add --file` requires the file to
# exist). trycmd has no fixture-sharing primitive, so each case still owns
# its sandbox — this just stamps one out.
#
# Usage:
#   dev/new-trycmd-test.sh tests/cmd/<command>/<sub>/<name> [--no-identity]
#
# Then write the fenced command session in the generated `<name>.md` and run
#   TRYCMD=overwrite cargo test --test trycmd_tests
# to capture the expected output.

set -euo pipefail

if [[ $# -lt 1 || "${1:-}" == -* ]]; then
	echo "usage: $0 tests/cmd/<command>/<sub>/<name> [--no-identity]" >&2
	exit 2
fi

base="${1%.md}"
base="${base%/}"
want_identity=1
[[ "${2:-}" == "--no-identity" ]] && want_identity=0

md="${base}.md"
in_dir="${base}.in"
out_dir="${base}.out"

if [[ -e "$md" || -e "$in_dir" || -e "$out_dir" ]]; then
	echo "error: '$base' already has a .md/.in/.out — refusing to overwrite" >&2
	exit 1
fi

mkdir -p "$in_dir" "$out_dir"

# The fixed test identity (also the `tests/cmd/export/bitwarden` fixture).
# Its content only has to parse as an age identity; the same value keeps the
# suite off the `age` key-generation API.
if [[ "$want_identity" == 1 ]]; then
	# `printf` (no trailing newline) so the file is byte-identical to the
	# existing `some-identity` fixtures.
	printf '# public key: age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd\nAGE-SECRET-KEY-14LGKWZMHDDWMLCL5EUU2FSQNEX33DZYR3F8VEPZSA670QRC3DA3SNSD535' \
		>"$in_dir/some-identity"
else
	# Keep the empty `.in` dir tracked in git.
	touch "$in_dir/.keep"
fi

# `.out` starts empty; trycmd compares only the files present in it, so an
# empty dir asserts stdout/exit only. The `.keep` keeps it in git.
touch "$out_dir/.keep"

cat >"$md" <<'TEMPLATE'
Describe what this case proves in a sentence or two.

```
$ pasejo store add --path store --name something
Store 'something' added at '[CWD]/store'

$ pasejo identity add --file some-identity --store something
Identity using file '[CWD]/some-identity' added

```
TEMPLATE

echo "created:"
echo "  $md"
echo "  $in_dir/$([[ "$want_identity" == 1 ]] && echo some-identity || echo .keep)"
echo "  $out_dir/.keep"
echo
echo "next: edit $md, then run"
echo "  TRYCMD=overwrite cargo test --test trycmd_tests"
