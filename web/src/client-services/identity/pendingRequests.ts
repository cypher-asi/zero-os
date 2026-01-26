/**
 * Pending Request Management
 *
 * This module provides backward compatibility re-exports from the shared IPC module.
 * New code should import from 'shared/ipc' directly.
 *
 * @deprecated Use PendingRequestQueue from 'shared/ipc' instead for new code
 */

import { PendingRequestQueue, type PendingRequest } from '@/shared/ipc';
import type { MinimalSupervisor } from '@/shared/types';

// Re-export types for backward compatibility
export type { PendingRequest };

// =============================================================================
// Shared request queue for IdentityServiceClient
// =============================================================================

const requestQueue = new PendingRequestQueue({ name: 'IdentityServiceClient' });

// =============================================================================
// Backward Compatible Functions
// =============================================================================

/** Counter for generating unique request IDs */
let requestCounter = 0;

/**
 * Generate a unique request ID.
 * @deprecated Use PendingRequestQueue.addRequest instead
 */
export function generateUniqueRequestId(tagHex: string): string {
  return `identity-${++requestCounter}-${tagHex}`;
}

/**
 * Add a pending request to the queue for its tag.
 * @deprecated Use PendingRequestQueue.addRequest instead
 */
export function addPendingRequest<T>(_tagHex: string, _request: PendingRequest<T>): void {
  // This is a legacy function - new code should use requestQueue.addRequest directly
  // For backward compatibility, we keep the function but it's not used anymore
  console.warn('[pendingRequests] addPendingRequest is deprecated, use PendingRequestQueue.addRequest');
}

/**
 * Remove a pending request by its unique ID (used for timeout cleanup).
 * @deprecated Use PendingRequestQueue internally handles this
 */
export function removePendingRequestById(_uniqueId: string): boolean {
  console.warn('[pendingRequests] removePendingRequestById is deprecated');
  return false;
}

/**
 * Ensure the IPC response callback is registered with the supervisor.
 * This uses the shared PendingRequestQueue internally.
 */
export function ensureCallbackRegistered(supervisor: MinimalSupervisor): void {
  requestQueue.register(supervisor);
}

/**
 * Add a request and return a promise that resolves when response arrives.
 * This is the new API that should be used.
 */
export function addRequestWithPromise<T>(tagHex: string, timeoutMs: number): Promise<T> {
  return requestQueue.addRequest<T>(tagHex, timeoutMs);
}
