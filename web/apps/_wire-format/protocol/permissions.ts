/**
 * App Protocol - Permission Protocol (03-security.md)
 *
 * Messages for permission management (Desktop/Supervisor -> Init).
 */

import {
  PROTOCOL_VERSION,
  TYPE_GRANT_REQUEST,
  TYPE_REVOKE_REQUEST,
  TYPE_PERMISSION_RESPONSE,
  TYPE_PERMISSION_LIST,
} from './types';
import { encodeEnvelope, decodeEnvelope, decodeString, decodeU8, decodeU32 } from './envelope';

// ============================================================================
// Permission Types
// ============================================================================

/**
 * Types of kernel objects that can be accessed via capabilities
 */
export type ObjectType = 'Endpoint' | 'Console' | 'Storage' | 'Network' | 'Process' | 'Memory';

/** Object type enum values (matching Rust kernel) */
export const OBJECT_TYPE = {
  Endpoint: 1,
  Console: 2,
  Storage: 3,
  Network: 4,
  Process: 5,
  Memory: 6,
} as const;

/**
 * Permission bits for capabilities
 */
export interface Permissions {
  read: boolean;
  write: boolean;
  grant: boolean;
}

/**
 * Encode permissions as a single byte
 */
export function encodePermissions(perms: Permissions): number {
  let byte = 0;
  if (perms.read) byte |= 0x01;
  if (perms.write) byte |= 0x02;
  if (perms.grant) byte |= 0x04;
  return byte;
}

/**
 * Decode permissions from a single byte
 */
export function decodePermissions(byte: number): Permissions {
  return {
    read: (byte & 0x01) !== 0,
    write: (byte & 0x02) !== 0,
    grant: (byte & 0x04) !== 0,
  };
}

/**
 * Capability request from an app's manifest
 */
export interface CapabilityRequest {
  /** Type of kernel object being requested */
  objectType: ObjectType;
  /** Permissions needed on this object */
  permissions: Permissions;
  /** Human-readable reason (shown to user in permission dialog) */
  reason: string;
  /** Whether this permission is required for the app to function */
  required: boolean;
}

// ============================================================================
// Grant Request (Desktop -> Init)
// ============================================================================

export interface GrantRequest {
  /** Target process to grant capability to */
  targetPid: number;
  /** Type of object being granted */
  objectType: ObjectType;
  /** Permissions to grant */
  permissions: Permissions;
  /** Reason (from AppManifest, for logging) */
  reason: string;
}

/**
 * Encode a grant request to bytes
 */
export function encodeGrantRequest(request: GrantRequest): Uint8Array {
  const reasonBytes = new TextEncoder().encode(request.reason);

  // Format: type_tag (1) + target_pid (4) + object_type (1) + perms (1) + reason_len (2) + reason
  const size = 1 + 4 + 1 + 1 + 2 + reasonBytes.length;
  const bytes = new Uint8Array(size);
  let offset = 0;

  // Type tag
  bytes[offset++] = TYPE_GRANT_REQUEST;

  // Target PID (little-endian u32)
  bytes[offset++] = request.targetPid & 0xff;
  bytes[offset++] = (request.targetPid >> 8) & 0xff;
  bytes[offset++] = (request.targetPid >> 16) & 0xff;
  bytes[offset++] = (request.targetPid >> 24) & 0xff;

  // Object type
  bytes[offset++] = OBJECT_TYPE[request.objectType];

  // Permissions
  bytes[offset++] = encodePermissions(request.permissions);

  // Reason (length-prefixed)
  bytes[offset++] = reasonBytes.length & 0xff;
  bytes[offset++] = (reasonBytes.length >> 8) & 0xff;
  bytes.set(reasonBytes, offset);

  return encodeEnvelope({
    version: PROTOCOL_VERSION,
    typeTag: TYPE_GRANT_REQUEST,
    payload: bytes,
  });
}

// ============================================================================
// Revoke Request (Desktop -> Init)
// ============================================================================

export interface RevokeRequest {
  /** Target process to revoke capability from */
  targetPid: number;
  /** Type of object being revoked */
  objectType: ObjectType;
}

/**
 * Encode a revoke request to bytes
 */
export function encodeRevokeRequest(request: RevokeRequest): Uint8Array {
  // Format: type_tag (1) + target_pid (4) + object_type (1)
  const bytes = new Uint8Array(6);

  bytes[0] = TYPE_REVOKE_REQUEST;

  // Target PID (little-endian u32)
  bytes[1] = request.targetPid & 0xff;
  bytes[2] = (request.targetPid >> 8) & 0xff;
  bytes[3] = (request.targetPid >> 16) & 0xff;
  bytes[4] = (request.targetPid >> 24) & 0xff;

  // Object type
  bytes[5] = OBJECT_TYPE[request.objectType];

  return encodeEnvelope({
    version: PROTOCOL_VERSION,
    typeTag: TYPE_REVOKE_REQUEST,
    payload: bytes,
  });
}

// ============================================================================
// Permission Response (Init -> Desktop)
// ============================================================================

export interface PermissionResponse {
  /** Whether the operation succeeded */
  success: boolean;
  /** Capability slot where cap was inserted (if success) */
  slot?: number;
  /** Error message (if !success) */
  error?: string;
}

/**
 * Decode a permission response from bytes
 */
export function decodePermissionResponse(data: Uint8Array): PermissionResponse | null {
  const envelope = decodeEnvelope(data);
  if (!envelope) return null;

  if (envelope.typeTag !== TYPE_PERMISSION_RESPONSE) {
    console.error(
      `Expected PERMISSION_RESPONSE (${TYPE_PERMISSION_RESPONSE}), got ${envelope.typeTag}`
    );
    return null;
  }

  const payload = envelope.payload;
  if (payload.length < 2) return null;

  const cursor = { pos: 1 }; // Skip type tag

  const success = decodeU8(payload, cursor);
  if (success === null) return null;

  if (success !== 0) {
    // Success: read slot
    const slot = decodeU32(payload, cursor);
    return { success: true, slot: slot ?? undefined };
  } else {
    // Failure: read error message
    const error = decodeString(payload, cursor);
    return { success: false, error: error ?? 'Unknown error' };
  }
}

// ============================================================================
// Permission List (Init -> Desktop)
// ============================================================================

export interface CapabilityInfo {
  /** Capability slot */
  slot: number;
  /** Object type */
  objectType: ObjectType;
  /** Permissions */
  permissions: Permissions;
}

export interface PermissionListResponse {
  /** Process ID */
  pid: number;
  /** List of granted capabilities */
  capabilities: CapabilityInfo[];
}

/**
 * Decode object type from numeric value
 */
function decodeObjectType(value: number): ObjectType | null {
  switch (value) {
    case 1:
      return 'Endpoint';
    case 2:
      return 'Console';
    case 3:
      return 'Storage';
    case 4:
      return 'Network';
    case 5:
      return 'Process';
    case 6:
      return 'Memory';
    default:
      return null;
  }
}

/**
 * Decode a permission list response from bytes
 */
export function decodePermissionList(data: Uint8Array): PermissionListResponse | null {
  const envelope = decodeEnvelope(data);
  if (!envelope) return null;

  if (envelope.typeTag !== TYPE_PERMISSION_LIST) {
    console.error(`Expected PERMISSION_LIST (${TYPE_PERMISSION_LIST}), got ${envelope.typeTag}`);
    return null;
  }

  const payload = envelope.payload;
  if (payload.length < 6) return null;

  const cursor = { pos: 1 }; // Skip type tag

  // PID
  const pid = decodeU32(payload, cursor);
  if (pid === null) return null;

  // Capability count
  const count = decodeU8(payload, cursor);
  if (count === null) return null;

  const capabilities: CapabilityInfo[] = [];

  for (let i = 0; i < count; i++) {
    const slot = decodeU32(payload, cursor);
    const objectTypeValue = decodeU8(payload, cursor);
    const permsValue = decodeU8(payload, cursor);

    if (slot === null || objectTypeValue === null || permsValue === null) {
      return null;
    }

    const objectType = decodeObjectType(objectTypeValue);
    if (objectType === null) continue;

    capabilities.push({
      slot,
      objectType,
      permissions: decodePermissions(permsValue),
    });
  }

  return { pid, capabilities };
}
