import { useState, useCallback } from 'react';
import { useIdentity, type UserId } from './useIdentity';

// =============================================================================
// Linked Account Types (mirrors zos-identity/src/keystore.rs and ipc.rs)
// =============================================================================

/**
 * Types of linkable credentials.
 * Corresponds to `CredentialType` in zos-identity/src/keystore.rs
 */
export type CredentialType = 'Email' | 'Phone' | 'OAuth' | 'WebAuthn';

/**
 * A linked external credential.
 * Corresponds to `LinkedCredential` in zos-identity/src/keystore.rs
 */
export interface LinkedCredential {
  /** Credential type */
  credentialType: CredentialType;
  /** Credential value (email address, phone number, etc.) */
  value: string;
  /** Whether this credential is verified */
  verified: boolean;
  /** When the credential was linked */
  linkedAt: number;
  /** When verification was completed */
  verifiedAt: number | null;
  /** Is this the primary credential of its type? */
  isPrimary: boolean;
}

/**
 * Result of successful email attachment.
 * Corresponds to `AttachEmailSuccess` in zos-identity/src/ipc.rs
 */
export interface AttachEmailSuccess {
  /** Verification required? */
  verificationRequired: boolean;
  /** Verification code sent to email (in dev mode only) */
  verificationCode: string | null;
}

/**
 * Linked Accounts state
 */
export interface LinkedAccountsState {
  /** List of linked credentials */
  credentials: LinkedCredential[];
  /** Loading state */
  isLoading: boolean;
  /** Error message */
  error: string | null;
}

/**
 * Hook return type
 */
export interface UseLinkedAccountsReturn {
  /** Current state */
  state: LinkedAccountsState;
  /** Get all linked credentials */
  getCredentials: () => Promise<LinkedCredential[]>;
  /** Get credentials by type */
  getCredentialsByType: (type: CredentialType) => LinkedCredential[];
  /** Get primary credential of a type */
  getPrimaryCredential: (type: CredentialType) => LinkedCredential | null;
  /** Attach an email credential */
  attachEmail: (email: string) => Promise<AttachEmailSuccess>;
  /** Verify an email credential */
  verifyEmail: (email: string, code: string) => Promise<void>;
  /** Remove a credential */
  removeCredential: (type: CredentialType, value: string) => Promise<void>;
  /** Set a credential as primary */
  setPrimaryCredential: (type: CredentialType, value: string) => Promise<void>;
  /** Refresh state */
  refresh: () => Promise<void>;
}

// =============================================================================
// IPC Message Types (from zos-identity/src/ipc.rs)
// =============================================================================

// user_msg::MSG_ATTACH_EMAIL = 0x7040
// user_msg::MSG_ATTACH_EMAIL_RESPONSE = 0x7041
// user_msg::MSG_GET_CREDENTIALS = 0x7042
// user_msg::MSG_GET_CREDENTIALS_RESPONSE = 0x7043

// =============================================================================
// Helpers
// =============================================================================

function generateMockVerificationCode(): string {
  return Math.random().toString().slice(2, 8);
}

function isValidEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

// =============================================================================
// Initial State
// =============================================================================

const INITIAL_STATE: LinkedAccountsState = {
  credentials: [],
  isLoading: false,
  error: null,
};

// =============================================================================
// Hook Implementation
// =============================================================================

export function useLinkedAccounts(): UseLinkedAccountsReturn {
  const identity = useIdentity();
  const [state, setState] = useState<LinkedAccountsState>(INITIAL_STATE);

  const getCredentials = useCallback(async (): Promise<LinkedCredential[]> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_GET_CREDENTIALS (0x7042)
      // Request: GetCredentialsRequest { user_id: UserId, credential_type: Option<CredentialType> }
      // Response: GetCredentialsResponse { credentials: Vec<LinkedCredential> }
      //
      // The identity service will read credentials from:
      // /home/{user_id}/.zos/credentials/credentials.json

      await new Promise(resolve => setTimeout(resolve, 200));

      setState(prev => ({
        ...prev,
        isLoading: false,
      }));

      return state.credentials;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to get credentials';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.credentials]);

  const getCredentialsByType = useCallback((type: CredentialType): LinkedCredential[] => {
    return state.credentials.filter(c => c.credentialType === type);
  }, [state.credentials]);

  const getPrimaryCredential = useCallback((type: CredentialType): LinkedCredential | null => {
    return state.credentials.find(c => c.credentialType === type && c.isPrimary) || null;
  }, [state.credentials]);

  const attachEmail = useCallback(async (email: string): Promise<AttachEmailSuccess> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    if (!isValidEmail(email)) {
      throw new Error('Invalid email format');
    }

    // Check if email already linked
    const existing = state.credentials.find(
      c => c.credentialType === 'Email' && c.value.toLowerCase() === email.toLowerCase()
    );
    if (existing) {
      throw new Error('Email already linked to this account');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC with MSG_ATTACH_EMAIL (0x7040)
      // Request: AttachEmailRequest { user_id: UserId, email: String }
      // Response: AttachEmailResponse { result: Result<AttachEmailSuccess, CredentialError> }
      //
      // The identity service will:
      // 1. Validate email format
      // 2. Check email not already linked
      // 3. Generate verification code
      // 4. Send verification email (or return code in dev mode)
      // 5. Store unverified credential

      await new Promise(resolve => setTimeout(resolve, 300));

      const now = Date.now();
      const isFirstEmail = !state.credentials.some(c => c.credentialType === 'Email');

      const newCredential: LinkedCredential = {
        credentialType: 'Email',
        value: email,
        verified: false, // Will be verified after code confirmation
        linkedAt: now,
        verifiedAt: null,
        isPrimary: isFirstEmail, // First email becomes primary
      };

      setState(prev => ({
        ...prev,
        credentials: [...prev.credentials, newCredential],
        isLoading: false,
      }));

      // In dev mode, return the verification code
      return {
        verificationRequired: true,
        verificationCode: generateMockVerificationCode(), // Only in dev mode
      };
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to attach email';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.credentials]);

  const verifyEmail = useCallback(async (email: string, code: string): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    const credential = state.credentials.find(
      c => c.credentialType === 'Email' && c.value.toLowerCase() === email.toLowerCase()
    );
    if (!credential) {
      throw new Error('Email not found');
    }

    if (credential.verified) {
      throw new Error('Email already verified');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC to verify email
      // This would validate the code against stored verification data

      await new Promise(resolve => setTimeout(resolve, 200));

      // Mock: Accept any 6-digit code
      if (code.length !== 6 || !/^\d+$/.test(code)) {
        throw new Error('Invalid verification code');
      }

      const now = Date.now();

      setState(prev => ({
        ...prev,
        credentials: prev.credentials.map(c =>
          c.credentialType === 'Email' && c.value.toLowerCase() === email.toLowerCase()
            ? { ...c, verified: true, verifiedAt: now }
            : c
        ),
        isLoading: false,
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to verify email';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.credentials]);

  const removeCredential = useCallback(async (type: CredentialType, value: string): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    const credential = state.credentials.find(
      c => c.credentialType === type && c.value === value
    );
    if (!credential) {
      throw new Error('Credential not found');
    }

    // Check if this is the only credential of this type and it's primary
    const sameTypeCredentials = state.credentials.filter(c => c.credentialType === type);
    if (sameTypeCredentials.length === 1 && credential.isPrimary) {
      // Allowed - removes the only credential
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC to remove credential
      // Update credentials.json in VFS

      await new Promise(resolve => setTimeout(resolve, 200));

      setState(prev => {
        const newCredentials = prev.credentials.filter(
          c => !(c.credentialType === type && c.value === value)
        );

        // If we removed the primary, make the first remaining one primary
        if (credential.isPrimary) {
          const firstOfType = newCredentials.find(c => c.credentialType === type);
          if (firstOfType) {
            firstOfType.isPrimary = true;
          }
        }

        return {
          ...prev,
          credentials: newCredentials,
          isLoading: false,
        };
      });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to remove credential';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.credentials]);

  const setPrimaryCredential = useCallback(async (type: CredentialType, value: string): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      throw new Error('No user logged in');
    }

    const credential = state.credentials.find(
      c => c.credentialType === type && c.value === value
    );
    if (!credential) {
      throw new Error('Credential not found');
    }

    if (!credential.verified) {
      throw new Error('Cannot set unverified credential as primary');
    }

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // TODO: Call supervisor IPC to update credentials
      // Update credentials.json in VFS

      await new Promise(resolve => setTimeout(resolve, 200));

      setState(prev => ({
        ...prev,
        credentials: prev.credentials.map(c => ({
          ...c,
          isPrimary: c.credentialType === type
            ? c.value === value
            : c.isPrimary,
        })),
        isLoading: false,
      }));
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to set primary credential';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMsg,
      }));
      throw err;
    }
  }, [identity?.state.currentUser?.id, state.credentials]);

  const refresh = useCallback(async (): Promise<void> => {
    const userId = identity?.state.currentUser?.id;
    if (!userId) {
      setState(INITIAL_STATE);
      return;
    }

    await getCredentials();
  }, [identity?.state.currentUser?.id, getCredentials]);

  return {
    state,
    getCredentials,
    getCredentialsByType,
    getPrimaryCredential,
    attachEmail,
    verifyEmail,
    removeCredential,
    setPrimaryCredential,
    refresh,
  };
}
