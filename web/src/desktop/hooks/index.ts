/**
 * Desktop Hooks
 *
 * Application-level hooks for business logic, state management, and service communication.
 *
 * These hooks are distinct from `components/Desktop/hooks/` which contains
 * UI rendering, DOM interaction, and animation hooks.
 *
 * Usage:
 * ```ts
 * import { useSupervisor, useIdentity, useMachineKeys } from '../desktop/hooks';
 * ```
 */

// Core context hooks
export { useSupervisor, useDesktopController, SupervisorProvider, DesktopControllerProvider } from './useSupervisor';
export type { Supervisor, DesktopController } from './useSupervisor';

// Identity hooks
export { useIdentity } from './useIdentity';
export { useIdentityServiceClient } from './useIdentityServiceClient';
export { useNeuralKey } from './useNeuralKey';
export type { NeuralShard, PublicIdentifiers, NeuralKeyGenerated, NeuralKeyState, UseNeuralKeyReturn } from './useNeuralKey';
export { useMachineKeys } from './useMachineKeys';
export type { MachineKeyCapabilities, MachineKeyRecord, MachineKeysState, KeyScheme, MachineKeyCapability, UseMachineKeysReturn } from './useMachineKeys';
export { useLinkedAccounts } from './useLinkedAccounts';
export type { CredentialType, LinkedCredential, LinkedAccountsState, UseLinkedAccountsReturn } from './useLinkedAccounts';

// ZID authentication hook
export { useZeroIdAuth } from './useZeroIdAuth';

// Tier status hook
export { useTierStatus } from './useTierStatus';
export type { UseTierStatusReturn } from './useTierStatus';

// Desktop/window hooks
export { useDesktops } from './useDesktops';
export { useWindows } from './useWindows';

// Permission hooks
export { usePermissions } from './usePermissions';

// Utility hooks
export { useKeyboardShortcuts } from './useKeyboardShortcuts';
export { useCopyToClipboard } from './useCopyToClipboard';
