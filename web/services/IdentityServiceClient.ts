/**
 * Identity Service Client - Backward Compatibility Re-exports
 *
 * This file exists for backward compatibility with code that imports directly from
 * 'services/IdentityServiceClient' rather than 'services/identity' or 'services'.
 *
 * The implementation has been split into smaller, more maintainable modules:
 * - services/identity/IdentityServiceClient.ts - Main client class
 * - services/identity/types.ts - TypeScript type definitions
 * - services/identity/errors.ts - Error class hierarchy
 * - services/identity/pendingRequests.ts - Request tracking utilities
 *
 * New code should import from 'services' (the index) or 'services/identity' directly.
 * This file is maintained to avoid breaking existing imports.
 */

export * from './identity';
