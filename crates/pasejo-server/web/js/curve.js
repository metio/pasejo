// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Whether 32 bytes encode a point of Curve25519's Edwards form, decided as
// ed25519-dalek's VerifyingKey::from_bytes decides it: the y coordinate is
// read modulo p, and a point exists when (y^2 - 1) / (d*y^2 + 1) is a square.
// WebCrypto may import any 32 bytes as a public key, so device records are
// checked here.

const P = (1n << 255n) - 19n;
const D = 37095705934669439343138083508754565189542113879843219016388785533085940283555n;

function power(base, exponent) {
  let result = 1n;
  let factor = base % P;
  let rest = exponent;
  while (rest > 0n) {
    if (rest & 1n) {
      result = (result * factor) % P;
    }
    factor = (factor * factor) % P;
    rest >>= 1n;
  }
  return result;
}

export function isOnCurve(bytes) {
  if (bytes.length !== 32) {
    return false;
  }
  let y = 0n;
  for (let index = 31; index >= 0; index--) {
    const byte = index === 31 ? bytes[index] & 0x7f : bytes[index];
    y = (y << 8n) | BigInt(byte);
  }
  y %= P;
  const y2 = (y * y) % P;
  const u = (y2 - 1n + P) % P;
  const v = (D * y2 + 1n) % P;
  const ratio = (u * power(v, P - 2n)) % P;
  if (ratio === 0n) {
    return true;
  }
  return power(ratio, (P - 1n) / 2n) === 1n;
}
