import { useEffect, useRef, useCallback, useMemo } from 'react';
import { useSupervisor, type Supervisor } from './useSupervisor';
import { useIdentityStore, selectCurrentUser } from '../../stores';
import { IdentityServiceClient, userIdToBigInt } from '../../services';

/**
 * Shared hook for accessing the IdentityServiceClient.
 *
 * Provides:
 * - A stable reference to the IdentityServiceClient (initialized when supervisor is available)
 * - The current user's ID as BigInt (for API calls)
 * - A helper to check if the client is ready
 *
 * This consolidates the duplicated client initialization pattern from
 * useMachineKeys, useLinkedAccounts, and useNeuralKey.
 */
export interface UseIdentityServiceClientReturn {
  /** The IdentityServiceClient instance, or null if not yet initialized */
  client: IdentityServiceClient | null;
  /** Current user ID as BigInt, or null if no user logged in */
  userId: bigint | null;
  /** Whether the client is ready for use */
  isReady: boolean;
  /** Get the client, throwing if not available */
  getClientOrThrow: () => IdentityServiceClient;
  /** Get the user ID, throwing if not available */
  getUserIdOrThrow: () => bigint;
}

/**
 * Hook to access the IdentityServiceClient with automatic initialization.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { client, userId, isReady, getClientOrThrow, getUserIdOrThrow } = useIdentityServiceClient();
 *
 *   const handleAction = useCallback(async () => {
 *     const client = getClientOrThrow();
 *     const userId = getUserIdOrThrow();
 *     await client.someMethod(userId, ...);
 *   }, [getClientOrThrow, getUserIdOrThrow]);
 *
 *   if (!isReady) return <Loading />;
 *   // ...
 * }
 * ```
 */
export function useIdentityServiceClient(): UseIdentityServiceClientReturn {
  const supervisor = useSupervisor();
  const currentUser = useIdentityStore(selectCurrentUser);

  // Stable reference to the client
  const clientRef = useRef<IdentityServiceClient | null>(null);

  // Initialize client when supervisor becomes available
  useEffect(() => {
    if (supervisor && !clientRef.current) {
      clientRef.current = new IdentityServiceClient(supervisor as unknown as Supervisor);
      console.log('[useIdentityServiceClient] IdentityServiceClient initialized');
    }
  }, [supervisor]);

  // Convert current user ID to BigInt
  const userId = useMemo(() => userIdToBigInt(currentUser?.id), [currentUser?.id]);

  // Check if ready
  const isReady = clientRef.current !== null && userId !== null;

  // Throwing getters for use in callbacks
  const getClientOrThrow = useCallback((): IdentityServiceClient => {
    const client = clientRef.current;
    if (!client) {
      throw new Error('Identity service client not available');
    }
    return client;
  }, []);

  const getUserIdOrThrow = useCallback((): bigint => {
    if (userId === null) {
      throw new Error('No user logged in');
    }
    return userId;
  }, [userId]);

  return {
    client: clientRef.current,
    userId,
    isReady,
    getClientOrThrow,
    getUserIdOrThrow,
  };
}
