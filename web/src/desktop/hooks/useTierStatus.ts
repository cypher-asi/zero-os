/**
 * useTierStatus Hook
 *
 * Fetches and caches the identity tier status (managed vs self-sovereign)
 * from the ZID server.
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import {
  IdentityServiceClient,
  type TierStatus,
  ServiceNotFoundError,
} from '@/client-services';
import { useSupervisor } from './useSupervisor';
import { useZeroIdAuth } from './useZeroIdAuth';
import {
  useIdentityStore,
  selectCurrentUser,
  selectTierStatus,
  type TierStatus as StoreTierStatus,
} from '@/stores';

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

// Cache tier status for 5 minutes
const TIER_CACHE_TTL_MS = 5 * 60 * 1000;

// =============================================================================
// Types
// =============================================================================

export interface UseTierStatusReturn {
  /** Current tier status (null if not loaded) */
  tierStatus: StoreTierStatus | null;
  /** Whether tier status is being fetched */
  isLoading: boolean;
  /** Error message if fetch failed */
  error: string | null;
  /** Refresh tier status from server */
  refresh: () => Promise<void>;
  /** Whether the identity is managed (can upgrade) */
  isManaged: boolean;
  /** Whether the identity is self-sovereign */
  isSelfSovereign: boolean;
}

// =============================================================================
// Hook Implementation
// =============================================================================

export function useTierStatus(): UseTierStatusReturn {
  // Use shared store for tier status
  const tierStatus = useIdentityStore(selectTierStatus);
  const setTierStatus = useIdentityStore((state) => state.setTierStatus);

  // Local state for loading/error
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const supervisor = useSupervisor();
  const currentUser = useIdentityStore(selectCurrentUser);
  const currentUserId = currentUser?.id ?? null;
  const { remoteAuthState } = useZeroIdAuth();

  // Track last fetch time for caching
  const lastFetchRef = useRef<number>(0);
  const initializedRef = useRef(false);

  // Create client instance when supervisor is available
  const clientRef = useRef<IdentityServiceClient | null>(null);
  if (supervisor && !clientRef.current) {
    clientRef.current = new IdentityServiceClient(supervisor);
  }

  const fetchTierStatus = useCallback(async (force = false) => {
    // Skip if not authenticated
    if (!currentUserId || !remoteAuthState?.accessToken) {
      return;
    }

    // Skip if cached and not forced
    const now = Date.now();
    if (!force && now - lastFetchRef.current < TIER_CACHE_TTL_MS && tierStatus) {
      return;
    }

    // Skip if no client
    if (!clientRef.current) {
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const serverEndpoint = remoteAuthState.serverEndpoint || DEFAULT_ZID_ENDPOINT;
      
      // Retry up to 3 times for service not found
      let status: TierStatus | null = null;
      let lastError: Error | null = null;
      const maxRetries = 3;
      const retryDelayMs = 1000;

      for (let attempt = 0; attempt < maxRetries; attempt++) {
        try {
          status = await clientRef.current.getTierStatus(
            currentUserId,
            remoteAuthState.accessToken,
            serverEndpoint
          );
          break;
        } catch (err) {
          lastError = err instanceof Error ? err : new Error('Unknown error');
          
          if (err instanceof ServiceNotFoundError) {
            if (attempt < maxRetries - 1) {
              await new Promise(resolve => setTimeout(resolve, retryDelayMs));
              continue;
            }
          }
          throw err;
        }
      }

      if (status) {
        // Convert API response to store format
        const storeTierStatus: StoreTierStatus = {
          tier: status.tier,
          authMethodsCount: status.auth_methods_count,
          canUpgrade: status.can_upgrade,
          upgradeRequirements: status.upgrade_requirements,
        };
        setTierStatus(storeTierStatus);
        lastFetchRef.current = now;
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to fetch tier status';
      setError(errorMsg);
      console.error('[useTierStatus] Fetch failed:', err);
    } finally {
      setIsLoading(false);
    }
  }, [currentUserId, remoteAuthState?.accessToken, remoteAuthState?.serverEndpoint, tierStatus, setTierStatus]);

  // Auto-fetch tier status when authenticated
  useEffect(() => {
    if (!remoteAuthState?.accessToken || !currentUserId || initializedRef.current) {
      return;
    }

    initializedRef.current = true;
    fetchTierStatus();
  }, [remoteAuthState?.accessToken, currentUserId, fetchTierStatus]);

  // Reset when user logs out
  useEffect(() => {
    if (!remoteAuthState?.accessToken) {
      setTierStatus(null);
      initializedRef.current = false;
      lastFetchRef.current = 0;
    }
  }, [remoteAuthState?.accessToken, setTierStatus]);

  const refresh = useCallback(async () => {
    await fetchTierStatus(true);
  }, [fetchTierStatus]);

  return {
    tierStatus,
    isLoading,
    error,
    refresh,
    isManaged: tierStatus?.tier === 'managed',
    isSelfSovereign: tierStatus?.tier === 'self_sovereign',
  };
}
