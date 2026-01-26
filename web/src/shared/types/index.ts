/**
 * Shared Types
 *
 * Canonical type definitions for the Zero OS frontend.
 * These types use camelCase and are intended for React components and hooks.
 */

// Supervisor types
export { type Supervisor, type MinimalSupervisor } from './supervisor';

// Identity types (UI format - camelCase)
export {
  type KeyScheme,
  type MachineKeyCapability,
  type MachineKeyCapabilities,
  type MachineKeyRecord,
  type MachineKeysState,
  type NeuralShard,
  type PublicIdentifiers,
  type NeuralKeyGenerated,
  type NeuralKeyState,
  type CredentialType,
  type LinkedCredential,
  type LinkedAccountsState,
} from './identity';
