// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Byte encodings of the wire format: unpadded base64url, lowercase hex
// identifiers, big-endian integers and UTF-8.

export class EncodingError extends Error {
  constructor(what) {
    super(`malformed ${what}`);
    this.name = "EncodingError";
  }
}

const ALPHABET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const LOOKUP = new Map([...ALPHABET].map((character, index) => [character, index]));

/** Encodes bytes as unpadded base64url. */
export function b64(bytes) {
  let text = "";
  let index = 0;
  for (; index + 3 <= bytes.length; index += 3) {
    const chunk = (bytes[index] << 16) | (bytes[index + 1] << 8) | bytes[index + 2];
    text +=
      ALPHABET[chunk >> 18] +
      ALPHABET[(chunk >> 12) & 63] +
      ALPHABET[(chunk >> 6) & 63] +
      ALPHABET[chunk & 63];
  }
  const rest = bytes.length - index;
  if (rest === 1) {
    const chunk = bytes[index] << 16;
    text += ALPHABET[chunk >> 18] + ALPHABET[(chunk >> 12) & 63];
  } else if (rest === 2) {
    const chunk = (bytes[index] << 16) | (bytes[index + 1] << 8);
    text += ALPHABET[chunk >> 18] + ALPHABET[(chunk >> 12) & 63] + ALPHABET[(chunk >> 6) & 63];
  }
  return text;
}

/**
 * Decodes unpadded base64url. Padding, other alphabets and non-zero
 * trailing bits are refused, as the Rust decoder refuses them.
 */
export function unb64(text) {
  if (typeof text !== "string" || text.length % 4 === 1) {
    throw new EncodingError("base64url");
  }
  const bytes = new Uint8Array(Math.floor((text.length * 3) / 4));
  let buffer = 0;
  let bits = 0;
  let position = 0;
  for (const character of text) {
    const value = LOOKUP.get(character);
    if (value === undefined) {
      throw new EncodingError("base64url");
    }
    buffer = (buffer << 6) | value;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      bytes[position++] = (buffer >> bits) & 0xff;
    }
    buffer &= (1 << bits) - 1;
  }
  if (buffer !== 0) {
    throw new EncodingError("base64url");
  }
  return bytes;
}

/** Decodes base64url that must hold exactly `length` bytes. */
export function unb64Fixed(text, length, what) {
  const bytes = unb64(text);
  if (bytes.length !== length) {
    throw new EncodingError(what);
  }
  return bytes;
}

const ID_PATTERN = /^[0-9a-f]{32}$/;

/** Whether `text` is an identifier: 32 lowercase hex digits. */
export function isId(text) {
  return typeof text === "string" && ID_PATTERN.test(text);
}

export function idBytes(text) {
  if (!isId(text)) {
    throw new EncodingError("identifier");
  }
  const bytes = new Uint8Array(16);
  for (let index = 0; index < 16; index++) {
    bytes[index] = Number.parseInt(text.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

export function hex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** A random identifier. */
export function randomId() {
  return hex(random(16));
}

export function random(length) {
  const bytes = new Uint8Array(length);
  globalThis.crypto.getRandomValues(bytes);
  return bytes;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export function utf8(text) {
  return encoder.encode(text);
}

export function fromUtf8(bytes) {
  try {
    return decoder.decode(bytes);
  } catch {
    throw new EncodingError("UTF-8");
  }
}

export function concat(...parts) {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    bytes.set(part, offset);
    offset += part.length;
  }
  return bytes;
}

/** The 8-byte big-endian form of a non-negative safe integer. */
export function be64(value) {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new EncodingError("integer");
  }
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, BigInt(value));
  return bytes;
}

export function equal(left, right) {
  if (left.length !== right.length) {
    return false;
  }
  let difference = 0;
  for (let index = 0; index < left.length; index++) {
    difference |= left[index] ^ right[index];
  }
  return difference === 0;
}

export async function sha256(bytes) {
  return new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", bytes));
}
