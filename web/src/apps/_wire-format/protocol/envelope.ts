/**
 * App Protocol - Envelope Encoding/Decoding
 *
 * Wire format (envelope):
 * ┌─────────┬──────────┬─────────────┬─────────────────────┐
 * │ version │ type_tag │ payload_len │       payload       │
 * │  (u8)   │   (u8)   │    (u16)    │      (bytes)        │
 * └─────────┴──────────┴─────────────┴─────────────────────┘
 */

import { PROTOCOL_VERSION } from './types';

export interface Envelope {
  version: number;
  typeTag: number;
  payload: Uint8Array;
}

/**
 * Encode an envelope to bytes
 */
export function encodeEnvelope(envelope: Envelope): Uint8Array {
  const payloadLen = envelope.payload.length;
  const bytes = new Uint8Array(4 + payloadLen);

  bytes[0] = envelope.version;
  bytes[1] = envelope.typeTag;
  bytes[2] = payloadLen & 0xff;
  bytes[3] = (payloadLen >> 8) & 0xff;
  bytes.set(envelope.payload, 4);

  return bytes;
}

/**
 * Decode an envelope from bytes
 */
export function decodeEnvelope(data: Uint8Array): Envelope | null {
  if (data.length < 4) {
    console.error('App protocol: data too short for envelope header');
    return null;
  }

  const version = data[0];
  if (version !== PROTOCOL_VERSION) {
    console.error(`App protocol: unknown version ${version}`);
    return null;
  }

  const typeTag = data[1];
  const payloadLen = data[2] | (data[3] << 8);

  if (data.length < 4 + payloadLen) {
    console.error(
      `App protocol: payload overflow (declared ${payloadLen}, available ${data.length - 4})`
    );
    return null;
  }

  const payload = data.slice(4, 4 + payloadLen);

  return { version, typeTag, payload };
}

// ============================================================================
// String Encoding Helpers
// ============================================================================

/**
 * Encode a string as length-prefixed UTF-8 (u16 length)
 */
export function encodeString(s: string): Uint8Array {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(s);
  const result = new Uint8Array(2 + bytes.length);
  result[0] = bytes.length & 0xff;
  result[1] = (bytes.length >> 8) & 0xff;
  result.set(bytes, 2);
  return result;
}

/**
 * Decode a length-prefixed string from data at the given cursor position
 */
export function decodeString(data: Uint8Array, cursor: { pos: number }): string | null {
  if (cursor.pos + 2 > data.length) {
    return null;
  }

  const len = data[cursor.pos] | (data[cursor.pos + 1] << 8);
  cursor.pos += 2;

  if (cursor.pos + len > data.length) {
    return null;
  }

  const bytes = data.slice(cursor.pos, cursor.pos + len);
  cursor.pos += len;

  const decoder = new TextDecoder();
  return decoder.decode(bytes);
}

/**
 * Decode a u8 from data at the given cursor position
 */
export function decodeU8(data: Uint8Array, cursor: { pos: number }): number | null {
  if (cursor.pos >= data.length) {
    return null;
  }
  const value = data[cursor.pos];
  cursor.pos += 1;
  return value;
}

/**
 * Decode a u32 (little-endian) from data at the given cursor position
 */
export function decodeU32(data: Uint8Array, cursor: { pos: number }): number | null {
  if (cursor.pos + 4 > data.length) {
    return null;
  }
  const value =
    data[cursor.pos] |
    (data[cursor.pos + 1] << 8) |
    (data[cursor.pos + 2] << 16) |
    (data[cursor.pos + 3] << 24);
  cursor.pos += 4;
  return value >>> 0; // Convert to unsigned
}

/**
 * Decode an optional char (0x00 = None, 0x01 + u32 = Some(char))
 */
export function decodeOptionalChar(data: Uint8Array, cursor: { pos: number }): string | null {
  const hasValue = decodeU8(data, cursor);
  if (hasValue === null) return null;

  if (hasValue === 0) {
    return null;
  }

  const code = decodeU32(data, cursor);
  if (code === null) return null;

  return String.fromCodePoint(code);
}
