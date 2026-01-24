/**
 * App Protocol - Input Events
 *
 * Input event encoding (UI -> App).
 */

import {
  PROTOCOL_VERSION,
  TYPE_BUTTON_PRESS,
  TYPE_TEXT_INPUT,
  TYPE_KEY_PRESS,
  TYPE_FOCUS_CHANGE,
} from './types';
import { encodeEnvelope, encodeString } from './envelope';

export type InputEvent =
  | { type: 'button'; name: string }
  | { type: 'text'; text: string }
  | { type: 'key'; keyCode: number; modifiers: number }
  | { type: 'focus'; gained: boolean };

/**
 * Create a button press input event
 */
export function buttonPress(name: string): InputEvent {
  return { type: 'button', name };
}

/**
 * Encode an InputEvent to bytes (for sending via IPC)
 */
export function encodeInputEvent(event: InputEvent): Uint8Array {
  let typeTag: number;
  let payloadParts: Uint8Array[];

  switch (event.type) {
    case 'button': {
      typeTag = TYPE_BUTTON_PRESS;
      payloadParts = [new Uint8Array([TYPE_BUTTON_PRESS]), encodeString(event.name)];
      break;
    }
    case 'text': {
      typeTag = TYPE_TEXT_INPUT;
      payloadParts = [new Uint8Array([TYPE_TEXT_INPUT]), encodeString(event.text)];
      break;
    }
    case 'key': {
      typeTag = TYPE_KEY_PRESS;
      const payload = new Uint8Array(6);
      payload[0] = TYPE_KEY_PRESS;
      payload[1] = event.keyCode & 0xff;
      payload[2] = (event.keyCode >> 8) & 0xff;
      payload[3] = (event.keyCode >> 16) & 0xff;
      payload[4] = (event.keyCode >> 24) & 0xff;
      payload[5] = event.modifiers;
      payloadParts = [payload];
      break;
    }
    case 'focus': {
      typeTag = TYPE_FOCUS_CHANGE;
      payloadParts = [new Uint8Array([TYPE_FOCUS_CHANGE, event.gained ? 1 : 0])];
      break;
    }
  }

  // Calculate total payload length
  const payloadLen = payloadParts.reduce((sum, p) => sum + p.length, 0);
  const payload = new Uint8Array(payloadLen);
  let offset = 0;
  for (const part of payloadParts) {
    payload.set(part, offset);
    offset += part.length;
  }

  return encodeEnvelope({
    version: PROTOCOL_VERSION,
    typeTag,
    payload,
  });
}
