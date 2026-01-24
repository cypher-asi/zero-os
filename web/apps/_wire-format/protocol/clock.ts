/**
 * App Protocol - Clock State
 *
 * Clock state decoder for the clock app.
 */

import { TYPE_CLOCK_STATE } from './types';
import { decodeEnvelope, decodeString, decodeU8 } from './envelope';

export interface ClockState {
  timeDisplay: string;
  dateDisplay: string;
  is24Hour: boolean;
  timezone: string;
}

/**
 * Decode ClockState from bytes (received via IPC)
 */
export function decodeClockState(data: Uint8Array): ClockState | null {
  const envelope = decodeEnvelope(data);
  if (!envelope) return null;

  if (envelope.typeTag !== TYPE_CLOCK_STATE) {
    console.error(`Expected CLOCK_STATE (${TYPE_CLOCK_STATE}), got ${envelope.typeTag}`);
    return null;
  }

  const payload = envelope.payload;
  if (payload.length === 0) {
    return null;
  }

  // Skip type tag in payload (byte 0)
  const cursor = { pos: 1 };

  const timeDisplay = decodeString(payload, cursor);
  if (timeDisplay === null) return null;

  const dateDisplay = decodeString(payload, cursor);
  if (dateDisplay === null) return null;

  const is24Hour = decodeU8(payload, cursor);
  if (is24Hour === null) return null;

  const timezone = decodeString(payload, cursor);
  if (timezone === null) return null;

  return {
    timeDisplay,
    dateDisplay,
    is24Hour: is24Hour !== 0,
    timezone,
  };
}
