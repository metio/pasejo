#!/usr/bin/env sh
# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

# Builds a statically linked musl binary of PACKAGE for the platform ARCH
# (amd64 or arm64, as container platforms name them) and copies it to OUTPUT.
# Runs as root on the Debian rust image of the Containerfiles, whatever the
# build machine's architecture.
set -eu

package="${1:?usage: dev/build-static.sh PACKAGE ARCH OUTPUT [VERSION]}"
arch="${2:?usage: dev/build-static.sh PACKAGE ARCH OUTPUT [VERSION]}"
output="${3:?usage: dev/build-static.sh PACKAGE ARCH OUTPUT [VERSION]}"
version="${4:-}"

case "${arch}" in
  amd64) machine=x86_64 ;;
  arm64) machine=aarch64 ;;
  *) echo "unsupported architecture ${arch}" >&2; exit 1 ;;
esac
target="${machine}-unknown-linux-musl"

# The C parts of dependencies (SQLite, aws-lc, ring) compile with clang
# against musl's headers for the target; glibc headers would leave references
# to symbols such as stat64 that musl does not define. Rust's self-contained
# musl objects and rust-lld link the result.
dpkg --add-architecture "${arch}"
apt-get update
apt-get install --yes --no-install-recommends clang llvm "musl-dev:${arch}"

variable="$(printf '%s' "${target}" | tr '[:lower:]-' '[:upper:]_')"
suffix="$(printf '%s' "${target}" | tr '-' '_')"
export "CC_${suffix}=clang"
export "AR_${suffix}=llvm-ar"
export "CFLAGS_${suffix}=-nostdlibinc -isystem /usr/include/${machine}-linux-musl"
export "CARGO_TARGET_${variable}_LINKER=rust-lld"
export "CARGO_TARGET_${variable}_RUSTFLAGS=-Clink-self-contained=yes"

if [ -n "${version}" ] && [ "${version}" != development ]; then
  dev/set-version.sh "${version}"
fi
rustup target add "${target}"
cargo build --release --target "${target}" --package "${package}"
cp "target/${target}/release/${package}" "${output}"
