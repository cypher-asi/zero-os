import { useState, useCallback } from 'react';

// =============================================================================
// ZERO ID Auth Types (mirrors zos-identity/src/session.rs)
// =============================================================================

/** Remote authentication state */
export interface RemoteAuthState {
  /** Remote authentication server endpoint */
  serverEndpoint: string;
  /** OAuth2/OIDC access token */
  accessToken: string;
  /** When the access token expires (timestamp) */
  tokenExpiresAt: number;
  /** Refresh token (if available) */
  refreshToken: string | null;
  /** Granted OAuth scopes */
  scopes: string[];
  /** User's ZERO ID key (truncated for display) */
  userKey: string;
}

/** Hook return type */
export interface UseZeroIdAuthReturn {
  /** Current remote auth state (null if not logged in) */
  remoteAuthState: RemoteAuthState | null;
  /** Whether authentication is in progress */
  isAuthenticating: boolean;
  /** Error message if any */
  error: string | null;
  /** Login with email and password */
  loginWithEmail: (email: string, password: string) => Promise<void>;
  /** Login with machine key challenge-response */
  loginWithMachineKey: () => Promise<void>;
  /** Logout from ZERO ID */
  logout: () => Promise<void>;
  /** Refresh the access token */
  refreshToken: () => Promise<void>;
  /** Get time remaining until token expires */
  getTimeRemaining: () => string;
  /** Check if token is expired */
  isTokenExpired: () => boolean;
}

// =============================================================================
// IPC Message Types (from zos-identity/src/ipc.rs)
// =============================================================================

// MSG_REMOTE_AUTH = 0x7020
// MSG_REMOTE_AUTH_RESPONSE = 0x7021

// =============================================================================
// Helpers
// =============================================================================

function generateMockToken(): string {
  const header = btoa(JSON.stringify({ alg: 'EdDSA', typ: 'JWT' }));
  const payload = btoa(JSON.stringify({
    sub: '1234567890',
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + 86400,
  }));
  const signature = Array.from(crypto.getRandomValues(new Uint8Array(32)))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
  return `${header}.${payload}.${signature}`;
}

function generateMockUserKey(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('').toUpperCase();
}

function formatTimeRemaining(expiresAt: number): string {
  const remaining = expiresAt - Date.now();
  if (remaining <= 0) {
    return 'Expired';
  }
  
  const hours = Math.floor(remaining / (60 * 60 * 1000));
  const minutes = Math.floor((remaining % (60 * 60 * 1000)) / (60 * 1000));
  
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

// =============================================================================
// Hook Implementation
// =============================================================================

export function useZeroIdAuth(): UseZeroIdAuthReturn {
  const [remoteAuthState, setRemoteAuthState] = useState<RemoteAuthState | null>(null);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loginWithEmail = useCallback(async (email: string, password: string) => {
    setIsAuthenticating(true);
    setError(null);

    try {
      // Validate input
      if (!email || !password) {
        throw new Error('Email and password are required');
      }

      const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
      if (!emailRegex.test(email)) {
        throw new Error('Invalid email format');
      }

      // TODO: Call supervisor IPC with MSG_REMOTE_AUTH (0x7020)
      // This would send credentials to a ZERO ID server
      await new Promise(resolve => setTimeout(resolve, 500));

      // Mock successful login
      const authState: RemoteAuthState = {
        serverEndpoint: 'https://auth.zero-id.io',
        accessToken: generateMockToken(),
        tokenExpiresAt: Date.now() + 24 * 60 * 60 * 1000, // 24 hours
        refreshToken: generateMockToken(),
        scopes: ['read', 'write', 'sync'],
        userKey: `UID-${generateMockUserKey().slice(0, 4)}-${generateMockUserKey().slice(0, 4)}-${generateMockUserKey().slice(0, 4)}`,
      };

      setRemoteAuthState(authState);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Authentication failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, []);

  const loginWithMachineKey = useCallback(async () => {
    setIsAuthenticating(true);
    setError(null);

    try {
      // TODO: Implement challenge-response authentication
      // 1. Request challenge from ZERO ID server
      // 2. Sign challenge with machine key
      // 3. Submit signed challenge for verification
      
      await new Promise(resolve => setTimeout(resolve, 500));

      const authState: RemoteAuthState = {
        serverEndpoint: 'https://auth.zero-id.io',
        accessToken: generateMockToken(),
        tokenExpiresAt: Date.now() + 24 * 60 * 60 * 1000,
        refreshToken: generateMockToken(),
        scopes: ['read', 'write', 'sync'],
        userKey: `UID-${generateMockUserKey().slice(0, 4)}-${generateMockUserKey().slice(0, 4)}-${generateMockUserKey().slice(0, 4)}`,
      };

      setRemoteAuthState(authState);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Machine key authentication failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, []);

  const logout = useCallback(async () => {
    setIsAuthenticating(true);
    setError(null);

    try {
      // TODO: Call supervisor IPC to invalidate remote session
      await new Promise(resolve => setTimeout(resolve, 200));

      setRemoteAuthState(null);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Logout failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, []);

  const refreshToken = useCallback(async () => {
    if (!remoteAuthState?.refreshToken) {
      throw new Error('No refresh token available');
    }

    setIsAuthenticating(true);
    setError(null);

    try {
      // TODO: Call supervisor IPC to refresh token
      await new Promise(resolve => setTimeout(resolve, 300));

      setRemoteAuthState(prev => prev ? {
        ...prev,
        accessToken: generateMockToken(),
        tokenExpiresAt: Date.now() + 24 * 60 * 60 * 1000,
        refreshToken: generateMockToken(),
      } : null);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Token refresh failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, [remoteAuthState?.refreshToken]);

  const getTimeRemaining = useCallback((): string => {
    if (!remoteAuthState) {
      return 'Not connected';
    }
    return formatTimeRemaining(remoteAuthState.tokenExpiresAt);
  }, [remoteAuthState]);

  const isTokenExpired = useCallback((): boolean => {
    if (!remoteAuthState) {
      return true;
    }
    return Date.now() >= remoteAuthState.tokenExpiresAt;
  }, [remoteAuthState]);

  return {
    remoteAuthState,
    isAuthenticating,
    error,
    loginWithEmail,
    loginWithMachineKey,
    logout,
    refreshToken,
    getTimeRemaining,
    isTokenExpired,
  };
}
