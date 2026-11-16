#!/usr/bin/env sh
# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

# Sets the version of every crate in the workspace and the exact version the
# published crates require of each other.
set -eu

version="${1:?usage: dev/set-version.sh VERSION}"
root="$(cd "$(dirname "$0")/.." && pwd)"

# sed without -i, whose argument differs between GNU and BSD sed.
rewrite() {
  file="$1"
  shift
  sed -E "$@" "${file}" > "${file}.version"
  mv "${file}.version" "${file}"
}

requirement="s/(pasejo-(protocol|client) = \{ path = \"[^\"]+\", version = \")=[^\"]*\"/\1=${version}\"/"
rewrite "${root}/Cargo.toml" -e "s/^version = \"[^\"]*\"$/version = \"${version}\"/" -e "${requirement}"
rewrite "${root}/crates/pasejo-client/Cargo.toml" -e "${requirement}"

metadata="$(cargo metadata --manifest-path "${root}/Cargo.toml" --no-deps --offline --format-version 1)"
for package in pasejo pasejo-protocol pasejo-client pasejo-server pasejo-bridge; do
  if ! printf '%s' "${metadata}" | grep -q "\"name\":\"${package}\",\"version\":\"${version}\""; then
    echo "${package} does not have version ${version}" >&2
    exit 1
  fi
done
if [ "$(grep -c "version = \"=${version}\"" "${root}/Cargo.toml" "${root}/crates/pasejo-client/Cargo.toml" | awk -F: '{ sum += $2 } END { print sum }')" != 3 ]; then
  echo "the published crates do not require each other at ${version}" >&2
  exit 1
fi
