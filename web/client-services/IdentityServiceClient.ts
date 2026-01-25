/**
 * Identity Service Client - Backward Compatibility Re-exports
 *
 * This file exists for backward compatibility with code that imports directly from
 * 'client-services/IdentityServiceClient' rather than 'client-services/identity' or 'client-services'.
 *
 * The implementation has been split into smaller, more maintainable modules:
 * - client-services/identity/IdentityServiceClient.ts - Main client class
 * - client-services/identity/types.ts - TypeScript type definitions
 * - client-services/identity/errors.ts - Error class hierarchy
 * - client-services/identity/pendingRequests.ts - Request tracking utilities
 *
 * New code should import from 'client-services' (the index) or 'client-services/identity' directly.
 * This file is maintained to avoid breaking existing imports.
 */

export * from './identity';
