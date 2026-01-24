import { useState, useCallback, useEffect, useRef } from 'react';
import { IdentityServiceClient, ZidTokens, ZidSession } from '../../services/IdentityServiceClient';
import { VfsStorageClient, formatUserId } from '../../services/VfsStorageClient';
import { useSupervisor } from './useSupervisor';
import { useIdentityStore, selectCurrentUser } from '../../stores';

// =============================================================================
// ZERO ID Auth Types (mirrors zos-identity/src/ipc.rs)
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
  /** Session ID from ZID server */
  sessionId: string;
}

/** Hook return type */
export interface UseZeroIdAuthReturn {
  /** Current remote auth state (null if not logged in) */
  remoteAuthState: RemoteAuthState | null;
  /** Whether authentication is in progress */
  isAuthenticating: boolean;
  /** Whether we're loading session from VFS */
  isLoadingSession: boolean;
  /** Error message if any */
  error: string | null;
  /** Login with email and password */
  loginWithEmail: (email: string, password: string) => Promise<void>;
  /** Login with machine key challenge-response */
  loginWithMachineKey: (zidEndpoint?: string) => Promise<void>;
  /** Enroll/register machine with ZID server */
  enrollMachine: (zidEndpoint?: string) => Promise<void>;
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
// Constants
// =============================================================================

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

// =============================================================================
// Helpers
// =============================================================================

/**
 * Get the canonical path for a user's ZID session.
 */
function getZidSessionPath(userId: bigint | string | number): string {
  return `/home/${formatUserId(userId)}/.zos/identity/zid_session.json`;
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
  const [isLoadingSession, setIsLoadingSession] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const supervisor = useSupervisor();
  const currentUser = useIdentityStore(selectCurrentUser);
  const currentUserId = currentUser?.id ?? null;

  // Track if we've initialized to avoid double-loading
  const initializedRef = useRef(false);

  // Create client instance when supervisor is available
  const clientRef = useRef<IdentityServiceClient | null>(null);
  if (supervisor && !clientRef.current) {
    clientRef.current = new IdentityServiceClient(supervisor);
  }

  // Load session from VFS cache on mount (or when user changes)
  useEffect(() => {
    if (!currentUserId || initializedRef.current) {
      setIsLoadingSession(false);
      return;
    }

    const loadSession = () => {
      try {
        const sessionPath = getZidSessionPath(currentUserId);
        const session = VfsStorageClient.readJsonSync<ZidSession>(sessionPath);

        if (session && session.expires_at > Date.now()) {
          // Valid session found, restore state
          setRemoteAuthState({
            serverEndpoint: session.zid_endpoint,
            accessToken: session.access_token,
            tokenExpiresAt: session.expires_at,
            refreshToken: session.refresh_token,
            scopes: ['read', 'write', 'sync'], // Default scopes
            sessionId: session.session_id,
          });
          console.log('[useZeroIdAuth] Restored session from VFS cache');
        } else if (session) {
          console.log('[useZeroIdAuth] Found expired session in VFS cache');
        }
      } catch (err) {
        console.warn('[useZeroIdAuth] Failed to load session from VFS:', err);
      } finally {
        setIsLoadingSession(false);
        initializedRef.current = true;
      }
    };

    loadSession();
  }, [currentUserId]);

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

      // TODO: Implement email/password login via IPC
      // This would be Phase 2 when we add email credential support
      throw new Error('Email login not yet implemented - use machine key login');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Authentication failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, []);

  const loginWithMachineKey = useCallback(
    async (zidEndpoint: string = DEFAULT_ZID_ENDPOINT) => {
      setIsAuthenticating(true);
      setError(null);

      try {
        if (!clientRef.current) {
          throw new Error('Supervisor not available - please wait for system to initialize');
        }
        if (!currentUserId) {
          throw new Error('You must be logged in locally before using Machine Key login');
        }

        // Call identity service to perform machine key login
        const tokens: ZidTokens = await clientRef.current.loginWithMachineKey(
          currentUserId,
          zidEndpoint
        );

        // Convert tokens to RemoteAuthState
        const authState: RemoteAuthState = {
          serverEndpoint: zidEndpoint,
          accessToken: tokens.access_token,
          tokenExpiresAt: Date.now() + tokens.expires_in * 1000, // expires_in is in seconds
          refreshToken: tokens.refresh_token,
          scopes: ['read', 'write', 'sync'],
          sessionId: tokens.session_id,
        };

        setRemoteAuthState(authState);
        console.log('[useZeroIdAuth] Machine key login successful');
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Machine key authentication failed';
        setError(errorMsg);
        throw err;
      } finally {
        setIsAuthenticating(false);
      }
    },
    [currentUserId]
  );

  const enrollMachine = useCallback(
    async (zidEndpoint: string = DEFAULT_ZID_ENDPOINT) => {
      setIsAuthenticating(true);
      setError(null);

      try {
        if (!clientRef.current) {
          throw new Error('Supervisor not available - please wait for system to initialize');
        }
        if (!currentUserId) {
          throw new Error('You must be logged in locally before enrolling machine');
        }

        // Call identity service to enroll machine with ZID server
        const tokens: ZidTokens = await clientRef.current.enrollMachine(currentUserId, zidEndpoint);

        // Convert tokens to RemoteAuthState (enrollment also logs you in)
        const authState: RemoteAuthState = {
          serverEndpoint: zidEndpoint,
          accessToken: tokens.access_token,
          tokenExpiresAt: Date.now() + tokens.expires_in * 1000,
          refreshToken: tokens.refresh_token,
          scopes: ['read', 'write', 'sync'],
          sessionId: tokens.session_id,
        };

        setRemoteAuthState(authState);
        console.log('[useZeroIdAuth] Machine enrollment successful');
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Machine enrollment failed';
        setError(errorMsg);
        throw err;
      } finally {
        setIsAuthenticating(false);
      }
    },
    [currentUserId]
  );

  const logout = useCallback(async () => {
    setIsAuthenticating(true);
    setError(null);

    try {
      // TODO: Implement logout via IPC to invalidate session server-side
      // For now, just clear local state
      setRemoteAuthState(null);
      console.log('[useZeroIdAuth] Logged out');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Logout failed';
      setError(errorMsg);
      throw err;
    } finally {
      setIsAuthenticating(false);
    }
  }, []);

  const refreshTokenFn = useCallback(async () => {
    if (!remoteAuthState?.refreshToken) {
      throw new Error('No refresh token available');
    }

    setIsAuthenticating(true);
    setError(null);

    try {
      // TODO: Implement token refresh via IPC when MSG_ZID_REFRESH is implemented
      throw new Error('Token refresh not yet implemented');
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
    isLoadingSession,
    error,
    loginWithEmail,
    loginWithMachineKey,
    enrollMachine,
    logout,
    refreshToken: refreshTokenFn,
    getTimeRemaining,
    isTokenExpired,
  };
}
