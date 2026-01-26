/**
 * App Protocol - Calculator State
 *
 * Calculator state decoder for the calculator app.
 */

import { TYPE_CALCULATOR_STATE } from './types';
import { decodeEnvelope, decodeString, decodeU8, decodeOptionalChar } from './envelope';

export interface CalculatorState {
  display: string;
  pendingOp: string | null;
  hasError: boolean;
  memoryIndicator: boolean;
}

/**
 * Decode CalculatorState from bytes (received via IPC)
 */
export function decodeCalculatorState(data: Uint8Array): CalculatorState | null {
  const envelope = decodeEnvelope(data);
  if (!envelope) return null;

  if (envelope.typeTag !== TYPE_CALCULATOR_STATE) {
    console.error(`Expected CALCULATOR_STATE (${TYPE_CALCULATOR_STATE}), got ${envelope.typeTag}`);
    return null;
  }

  const payload = envelope.payload;
  if (payload.length === 0) {
    return null;
  }

  // Skip type tag in payload (byte 0)
  const cursor = { pos: 1 };

  const display = decodeString(payload, cursor);
  if (display === null) return null;

  const pendingOp = decodeOptionalChar(payload, cursor);

  const hasError = decodeU8(payload, cursor);
  if (hasError === null) return null;

  const memoryIndicator = decodeU8(payload, cursor);
  if (memoryIndicator === null) return null;

  return {
    display,
    pendingOp,
    hasError: hasError !== 0,
    memoryIndicator: memoryIndicator !== 0,
  };
}
