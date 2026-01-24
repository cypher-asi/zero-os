/**
 * App Protocol - Settings State
 *
 * Settings state decoder for the settings app.
 */

import { TYPE_SETTINGS_STATE } from './types';
import { decodeEnvelope, decodeString, decodeU8 } from './envelope';

export interface SettingsState {
  activeArea: number; // 0=General, 1=Identity, 2=Permissions, 3=Theme
  activeItem: string;
  // General
  timeFormat24h: boolean;
  timezone: string;
  // Theme
  theme: string;
  accent: string;
  background: string;
  // Identity summary
  hasNeuralKey: boolean;
  machineKeyCount: number;
  linkedAccountCount: number;
  // Permissions summary
  runningProcessCount: number;
  totalCapabilityCount: number;
}

/**
 * Decode SettingsState from bytes (received via IPC)
 */
export function decodeSettingsState(data: Uint8Array): SettingsState | null {
  const envelope = decodeEnvelope(data);
  if (!envelope) return null;

  if (envelope.typeTag !== TYPE_SETTINGS_STATE) {
    console.error(`Expected SETTINGS_STATE (${TYPE_SETTINGS_STATE}), got ${envelope.typeTag}`);
    return null;
  }

  const payload = envelope.payload;
  if (payload.length === 0) {
    return null;
  }

  // Skip type tag in payload (byte 0)
  const cursor = { pos: 1 };

  const activeArea = decodeU8(payload, cursor);
  if (activeArea === null) return null;

  const activeItem = decodeString(payload, cursor);
  if (activeItem === null) return null;

  // General settings
  const timeFormat24h = decodeU8(payload, cursor);
  if (timeFormat24h === null) return null;

  const timezone = decodeString(payload, cursor);
  if (timezone === null) return null;

  // Theme settings
  const theme = decodeString(payload, cursor);
  if (theme === null) return null;

  const accent = decodeString(payload, cursor);
  if (accent === null) return null;

  const background = decodeString(payload, cursor);
  if (background === null) return null;

  // Identity summary
  const hasNeuralKey = decodeU8(payload, cursor);
  if (hasNeuralKey === null) return null;

  const machineKeyCount = decodeU8(payload, cursor);
  if (machineKeyCount === null) return null;

  const linkedAccountCount = decodeU8(payload, cursor);
  if (linkedAccountCount === null) return null;

  // Permissions summary
  const runningProcessCount = decodeU8(payload, cursor);
  if (runningProcessCount === null) return null;

  const totalCapabilityCount = decodeU8(payload, cursor);
  if (totalCapabilityCount === null) return null;

  return {
    activeArea,
    activeItem,
    timeFormat24h: timeFormat24h !== 0,
    timezone,
    theme,
    accent,
    background,
    hasNeuralKey: hasNeuralKey !== 0,
    machineKeyCount,
    linkedAccountCount,
    runningProcessCount,
    totalCapabilityCount,
  };
}
