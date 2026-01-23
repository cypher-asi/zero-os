import { useState, useCallback } from 'react';
import { useIdentity, type UserId } from './useIdentity';

// =============================================================================
// Machine Key Types (mirrors zos-identity/src/keystore.rs and ipc.rs)
// =============================================================================

/**
 * Capabilities of machine-level keys.
 * Corresponds to `MachineKeyCapabilities` in zos-identity/src/keystore.rs
 */
export interface MachineKeyCapabilities {
  /** Can sign authentication challenges */
  canAuthenticate: boolean;
  /** Can encrypt/decrypt local data */
  canEncrypt: boolean;
  /** Can sign messages on behalf of user */
  canSignMessages: boolean;
  /** Can authorize new machines */
  canAuthorizeMachines: boolean;
  /** Can revoke other machines */
  canRevokeMachines: boolean;
  /** Expiry time (null = no expiry) */
  expiresAt: number | null;
}

/**
 * Per-machine key record.
 * Corresponds to `MachineKeyRecord` in zos-identity/src/keystore.rs
 */
export interface MachineKeyRecord {
  /** Machine ID (128-bit as hex string) */
  machineId: string;
  /** Machine-specific signing public key (Ed25519, hex) */
  signingPublicKey: string;
  /** Machine-specific encryption public key (X25519, hex) */
  encryptionPublicKey: string;
  /** When this machine was authorized */
  authorizedAt: number;
  /** Who authorized this machine (user_id or machine_id as hex) */
  authorizedBy: string;
  /** Machine capabilities */
  capabilities: MachineKeyCapabilities;
  /** Human-readable machine name */
  machineName: string | null;
  /** Last seen timestamp */
  lastSeenAt: number;
}

/**
 * Machine Keys state
 */
export interface MachineKeysState {
  /** List of machine key records */
  machines: MachineKeyRecord[];
  /** Current machine ID (if applicable) */
  currentMachineId: string | null;
  /** Loading state */
  isLoading: boolean;
  /** Error message */
  error: string | null;
}

/**
 * Hook return type
 */
export interface UseMachineKeysReturn {
  /** Current state */
  state: MachineKeysState;
  /** List all machine keys for current user */
  listMachineKeys: () => Promise<MachineKeyRecord[]>;
  /** Get a specific machine key */
  getMachineKey: (machineId: string) => Promise<MachineKeyRecord | null>;
  /** Create a new machine key */
  createMachineKey: (machineName?: string, capabilities?: Partial<MachineKeyCapabilities>) => Promise<MachineKeyRecord>;
  /** Revoke a machine key */
  revokeMachineKey: (machineId: string) => Promise<void>;
  /** Rotate a machine key (new epoch) */
  rotateMachineKey: (machineId: string) => Promise<MachineKeyRecord>;
  /** Refresh state */
  refresh: () => Promise<void>;
}

// =============================================================================
// IPC Message Types (from zos-identity/src/ipc.rs)
// =============================================================================

// key_msg::MSG_CREATE_MACHINE_KEY = 0x7060
// key_msg::MSG_CREATE_MACHINE_KEY_RESPONSE = 0x7061
// key_msg::MSG_LIST_MACHINE_KEYS = 0x7062
// key_msg::MSG_LIST_MACHINE_KEYS_RESPONSE = 0x7063
// key_msg::MSG_GET_MACHINE_KEY = 0x7064
// key_msg::MSG_GET_MACHINE_KEY_RESPONSE = 0x7065
// key_msg::MSG_REVOKE_MACHINE_KEY = 0x7066
// key_msg::MSG_REVOKE_MACHINE_KEY_RESPONSE = 0x7067
// key_msg::MSG_ROTATE_MACHINE_KEY = 0x7068
// key_msg::MSG_ROTATE_MACHINE_KEY_RESPONSE = 0x7069

// =============================================================================
// Helpers
// =============================================================================

function generateMockHexKey(length: number): string {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

function generateMockMachineId(): string {
  return generateMockHexKey(16); // 128-bit
}

function getDefaultCapabilities(): MachineKeyCapabilities {
  return {
    canAuthenticate: true,
    canEncrypt: true,
    canSignMessages: false,
    canAuthorizeMachines: false,
    canRevokeMachines: false,
    expiresAt: null,
  };
}

// =============================================================================
// Initial State
// =============================================================================

const INITIAL_STATE: MachineKeysState = {
  machines: [],
  currentMachineId: null,
  isLoading: false,
  error: null,
};

// =============================================================================
// Hook Implementation
// =============================================================================

export function useMachineKeys(): UseMachineKeysReturn {
  const identity = useIdentity();
  const [state, setState] = useState<MachineKeysState>(INITIAL_STATE);

  const listMachineKeys = useCallback(async (): Promise<MachineKeyRecord[]> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_LIST_MACHINE_KEYS (0x7062)
      // Request: ListMachineKeysRequest { user_id: UserId }
      // Response: ListMachineKeysResponse { machines: Vec<MachineKeyRecord> }
      //
      // The identity service will read machine keys from:
      // /home/{user_id}/.zos/identity/machine/*.json

      await new Promise(resolve => setTimeout(resolve, 200));

      // Return current state machines (mock)
      setState(prev => ({
        ...prev,
        isLoading: false,
      }));

      return state.machines;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to list machine keys';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.machines]);

  const getMachineKey = useCallback(async (machineId: string): Promise<MachineKeyRecord | null> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    try {
      // TODO: Call supervisor IPC with MSG_GET_MACHINE_KEY (0x7064)
      // Request: GetMachineKeyRequest { user_id: UserId, machine_id: u128 }
      // Response: GetMachineKeyResponse { result: Result<Option<MachineKeyRecord>, KeyError> }

      await new Promise(resolve => setTimeout(resolve, 100));

      // Mock: Find in current state
      return state.machines.find(m => m.machineId === machineId) || null;
    } catch {
      return null;
    }
  }, [identity?.state.currentUser?.id, state.machines]);

  const createMachineKey = useCallback(async (
    machineName?: string,
    capabilities?: Partial<MachineKeyCapabilities>
  ): Promise<MachineKeyRecord> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_CREATE_MACHINE_KEY (0x7060)
      // Request: CreateMachineKeyRequest {
      //   user_id: UserId,
      //   machine_name: Option<String>,
      //   capabilities: MachineKeyCapabilities,
      //   signing_public_key: [u8; 32],
      //   encryption_public_key: [u8; 32],
      // }
      // Response: CreateMachineKeyResponse { result: Result<MachineKeyRecord, KeyError> }
      //
      // Note: The actual key generation happens client-side or in the identity service.
      // The public keys are sent in the request.

      await new Promise(resolve => setTimeout(resolve, 300));

      const now = Date.now();
      const machineId = generateMockMachineId();

      const newMachine: MachineKeyRecord = {
        machineId,
        signingPublicKey: generateMockHexKey(32),
        encryptionPublicKey: generateMockHexKey(32),
        authorizedAt: now,
        authorizedBy: userId,
        capabilities: {
          ...getDefaultCapabilities(),
          ...capabilities,
        },
        machineName: machineName || null,
        lastSeenAt: now,
      };

      setState(prev => ({
        ...prev,
        machines: [...prev.machines, newMachine],
        currentMachineId: prev.currentMachineId || machineId,
        isLoading: false,
      }));

      return newMachine;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to create machine key';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id]);

  const revokeMachineKey = useCallback(async (machineId: string): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    // Cannot revoke current machine
    if (machineId === state.currentMachineId) {
      throw new Error('Cannot revoke the current machine key');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_REVOKE_MACHINE_KEY (0x7066)
      // Request: RevokeMachineKeyRequest { user_id: UserId, machine_id: u128 }
      // Response: RevokeMachineKeyResponse { result: Result<(), KeyError> }
      //
      // The identity service will delete:
      // /home/{user_id}/.zos/identity/machine/{machine_id}.json

      await new Promise(resolve => setTimeout(resolve, 200));

      setState(prev => ({
        ...prev,
        machines: prev.machines.filter(m => m.machineId !== machineId),
        isLoading: false,
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to revoke machine key';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.currentMachineId]);

  const rotateMachineKey = useCallback(async (machineId: string): Promise<MachineKeyRecord> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    const existingMachine = state.machines.find(m => m.machineId === machineId);
    if (!existingMachine) {
      throw new Error('Machine not found');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_ROTATE_MACHINE_KEY (0x7068)
      // Request: RotateMachineKeyRequest {
      //   user_id: UserId,
      //   machine_id: u128,
      //   new_signing_public_key: [u8; 32],
      //   new_encryption_public_key: [u8; 32],
      // }
      // Response: RotateMachineKeyResponse { result: Result<MachineKeyRecord, KeyError> }
      //
      // This increments the key epoch and stores new public keys.

      await new Promise(resolve => setTimeout(resolve, 300));

      const now = Date.now();
      const rotatedMachine: MachineKeyRecord = {
        ...existingMachine,
        signingPublicKey: generateMockHexKey(32),
        encryptionPublicKey: generateMockHexKey(32),
        lastSeenAt: now,
      };

      setState(prev => ({
        ...prev,
        machines: prev.machines.map(m =>
          m.machineId === machineId ? rotatedMachine : m
        ),
        isLoading: false,
      }));

      return rotatedMachine;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to rotate machine key';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.machines]);

  const refresh = useCallback(async (): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      setState(INITIAL_STATE);
      return;
    }

    await listMachineKeys();
  }, [identity?.state.currentUser?.id, listMachineKeys]);

  return {
    state,
    listMachineKeys,
    getMachineKey,
    createMachineKey,
    revokeMachineKey,
    rotateMachineKey,
    refresh,
  };
}
