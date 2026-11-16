// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Device records, their fingerprints and the text rules the log applies.

import { EncodingError, concat, hex, isId, sha256, unb64, unb64Fixed, utf8 } from "./encoding.js";

export const MAX_LABEL_LEN = 200;
export const MAX_RECIPIENT_LEN = 2048;

// Unicode White_Space, which Rust's str::trim strips. JavaScript's trim
// also strips U+FEFF, which is no whitespace there.
const WHITE_SPACE = /^[\t\n\v\f\r \x85\xa0\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}]*$/u;
// Unicode general category Cc, which Rust's char::is_control matches.
const CONTROL = /[\x00-\x1f\x7f-\x9f]/u;

/** Whether `text` is empty or whitespace only, as Rust's trim sees it. */
export function blank(text) {
  return WHITE_SPACE.test(text);
}

export function hasControl(text) {
  return CONTROL.test(text);
}

/** The length of `text` in UTF-8 bytes. */
export function byteLength(text) {
  return utf8(text).length;
}

/** A device or team name: not blank, at most 200 bytes, no controls. */
export function validName(text) {
  return typeof text === "string" && !blank(text) && byteLength(text) <= 200 && !hasControl(text);
}

export function validRecipient(kind, recipient) {
  if (typeof recipient !== "string") {
    return false;
  }
  if (kind === "hpke") {
    try {
      return unb64(recipient).length === 32;
    } catch {
      return false;
    }
  }
  if (kind === "age") {
    return (
      recipient.length > 0 &&
      recipient.length <= MAX_RECIPIENT_LEN &&
      /^[\x20-\x7e]*$/.test(recipient) &&
      (recipient.startsWith("age1") || recipient.startsWith("ssh-"))
    );
  }
  return false;
}

/** Whether the record's label and recipient are acceptable. */
export function validRecord(record) {
  return (
    !blank(record.label) &&
    byteLength(record.label) <= MAX_LABEL_LEN &&
    !hasControl(record.label) &&
    validRecipient(record.kind, record.recipient)
  );
}

/** Parses a device record from its JSON form. */
export function parseRecord(value) {
  if (typeof value !== "object" || value === null) {
    throw new EncodingError("device record");
  }
  const { id, user, kind, recipient, signing_key: signingKey, label } = value;
  if (!isId(id) || !isId(user) || (kind !== "age" && kind !== "hpke")) {
    throw new EncodingError("device record");
  }
  if (typeof recipient !== "string" || typeof label !== "string") {
    throw new EncodingError("device record");
  }
  return {
    id,
    user,
    kind,
    recipient,
    signingKey: unb64Fixed(signingKey, 32, "signing key"),
    label,
  };
}

function groups(hash) {
  const text = hex(hash.subarray(0, 20));
  return text.match(/.{8}/g).join(" ");
}

/** The fingerprint people compare before approving a device. */
export async function fingerprint(kind, recipient, signingKey) {
  const zero = new Uint8Array([0]);
  return groups(await sha256(concat(utf8(kind), zero, utf8(recipient), zero, signingKey)));
}

export function recordFingerprint(record) {
  return fingerprint(record.kind, record.recipient, record.signingKey);
}

/** The fingerprint of an organization: its genesis hash, grouped. */
export function orgFingerprint(genesisHash) {
  return groups(genesisHash);
}

/** The device identifier under which the recovery recipient is wrapped. */
export async function recoveryId(recipient) {
  const hash = await sha256(concat(utf8("pasejo recovery v1"), new Uint8Array([0]), utf8(recipient)));
  return hex(hash.subarray(0, 16));
}

/** Compares a typed fingerprint with the expected one, ignoring spacing and case. */
export function fingerprintMatches(typed, expected) {
  const normalize = (text) => text.replace(/\s+/g, "").toLowerCase();
  return normalize(typed) !== "" && normalize(typed) === normalize(expected);
}
