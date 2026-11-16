// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  EncodingError,
  b64,
  be64,
  concat,
  equal,
  fromUtf8,
  hex,
  idBytes,
  isId,
  randomId,
  sha256,
  unb64,
  unb64Fixed,
  utf8,
} from "../js/encoding.js";

test("base64url round-trips every length and matches the platform encoder", () => {
  for (let length = 0; length < 70; length++) {
    const bytes = Uint8Array.from({ length }, (_, index) => (index * 37 + length) & 0xff);
    const text = b64(bytes);
    assert.equal(text, Buffer.from(bytes).toString("base64url"));
    assert.deepEqual(unb64(text), bytes);
  }
});

test("base64url refuses padding, other alphabets, impossible lengths and trailing bits", () => {
  for (const text of ["AA==", "A+", "A/", "A", "AAAAA", "AB", "AAB", "A A", 7, null]) {
    assert.throws(() => unb64(text), EncodingError, String(text));
  }
  assert.deepEqual(unb64("AA"), new Uint8Array([0]));
  assert.deepEqual(unb64("AAA"), new Uint8Array([0, 0]));
});

test("fixed-length decoding checks the length", () => {
  assert.equal(unb64Fixed(b64(new Uint8Array(32)), 32, "key").length, 32);
  assert.throws(() => unb64Fixed(b64(new Uint8Array(31)), 32, "key"), /malformed key/);
});

test("identifiers are 32 lowercase hex digits", () => {
  const id = randomId();
  assert.ok(isId(id));
  assert.equal(hex(idBytes(id)), id);
  for (const bad of ["", "0f".repeat(15), "0F".repeat(16), "zz".repeat(16), "0f".repeat(17), 5]) {
    assert.equal(isId(bad), false, String(bad));
    assert.throws(() => idBytes(bad), EncodingError);
  }
});

test("integers are 8 bytes big-endian and must be safe and non-negative", () => {
  assert.deepEqual(be64(0x0102), Uint8Array.from([0, 0, 0, 0, 0, 0, 1, 2]));
  assert.deepEqual(be64(Number.MAX_SAFE_INTEGER), Uint8Array.from([0, 0x1f, 255, 255, 255, 255, 255, 255]));
  for (const bad of [-1, 1.5, 2 ** 53, Number.NaN]) {
    assert.throws(() => be64(bad), EncodingError);
  }
});

test("UTF-8 decoding is strict", () => {
  assert.equal(fromUtf8(utf8("\xfc \u{1f642}")), "\xfc \u{1f642}");
  assert.throws(() => fromUtf8(Uint8Array.from([0xc3])), EncodingError);
});

test("helpers concatenate, compare and hash", async () => {
  assert.deepEqual(concat(Uint8Array.of(1), Uint8Array.of(), Uint8Array.of(2, 3)), Uint8Array.of(1, 2, 3));
  assert.ok(equal(Uint8Array.of(1, 2), Uint8Array.of(1, 2)));
  assert.ok(!equal(Uint8Array.of(1, 2), Uint8Array.of(1, 3)));
  assert.ok(!equal(Uint8Array.of(1), Uint8Array.of(1, 2)));
  assert.equal(hex(await sha256(utf8("abc"))), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
});
