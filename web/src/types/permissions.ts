/**
 * Shared Permission Types
 *
 * These types define the capability-based security model used throughout
 * the application. They match the 03-security.md specification.
 */

// =============================================================================
// Core Types
// =============================================================================

/**
 * Types of kernel objects that can be accessed via capabilities
 */
export type ObjectType = 'Endpoint' | 'Console' | 'Storage' | 'Network' | 'Process' | 'Memory';

/**
 * Permission bits for capabilities
 */
export interface Permissions {
  read: boolean;
  write: boolean;
  grant: boolean;
}

/**
 * Information about a granted capability
 */
export interface CapabilityInfo {
  /** Capability slot in the process's CSpace */
  slot: number;
  /** Object type */
  objectType: ObjectType;
  /** Permissions */
  permissions: Permissions;
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

/**
 * App manifest information
 */
export interface AppManifest {
  /** Unique app identifier (e.g., "com.example.myapp") */
  id: string;
  /** Display name */
  name: string;
  /** Version string */
  version: string;
  /** Description (optional - used in permission dialogs) */
  description?: string;
  /** Requested capabilities (optional - used in permission dialogs) */
  capabilities?: CapabilityRequest[];
  /** Whether this is a factory (trusted) app */
  isFactory?: boolean;
}
