/**
 * useWalletAccountWatcher - Watches for MetaMask account changes
 *
 * Automatically disconnects the ZID session when the user switches
 * to a different wallet address in MetaMask.
 *
 * This ensures the session always matches the active wallet address,
 * preventing confusion and security issues.
 */

import { useEffect, useRef } from 'react';
import { useIdentityStore, selectRemoteAuthState } from '@/stores';
import { useZeroIdAuth } from './useZeroIdAuth';
// Import ethereum types
import '@/types/ethereum.d.ts';

/**
 * Hook that watches for wallet account changes and auto-disconnects
 * when the wallet address changes.
 *
 * Only active when:
 * 1. There's an active ZID session
 * 2. The session was authenticated via wallet
 * 3. MetaMask (or compatible wallet) is available
 */
export function useWalletAccountWatcher(): void {
  const remoteAuthState = useIdentityStore(selectRemoteAuthState);
  const { disconnect } = useZeroIdAuth();
  
  // Track the connected address to detect changes
  const connectedAddressRef = useRef<string | null>(null);
  
  useEffect(() => {
    // Only watch if logged in via wallet
    if (remoteAuthState?.loginType !== 'wallet') {
      connectedAddressRef.current = null;
      return;
    }
    
    // Check if ethereum provider is available
    if (!window.ethereum) {
      return;
    }
    
    // Get the stored wallet address (normalized to lowercase)
    const storedAddress = remoteAuthState.authIdentifier?.toLowerCase() ?? null;
    connectedAddressRef.current = storedAddress;
    
    /**
     * Handle account changes from MetaMask
     */
    const handleAccountsChanged = async (accounts: string[]) => {
      const currentStoredAddress = connectedAddressRef.current;
      
      // If no stored address, nothing to compare
      if (!currentStoredAddress) {
        return;
      }
      
      // Get the new active address (normalized)
      const newAddress = accounts.length > 0 ? accounts[0].toLowerCase() : null;
      
      // If address changed or wallet disconnected, disconnect the session
      if (newAddress !== currentStoredAddress) {
        console.log('[WalletWatcher] Wallet address changed:', currentStoredAddress, '→', newAddress);
        
        try {
          await disconnect();
          console.log('[WalletWatcher] Session disconnected due to wallet change');
        } catch (err) {
          console.error('[WalletWatcher] Failed to disconnect session:', err);
        }
        
        // Clear the tracked address
        connectedAddressRef.current = null;
      }
    };
    
    // Subscribe to account changes
    window.ethereum.on('accountsChanged', handleAccountsChanged);
    
    console.log('[WalletWatcher] Watching wallet:', storedAddress?.slice(0, 10) + '...');
    
    // Cleanup listener on unmount or when session changes
    return () => {
      if (window.ethereum) {
        window.ethereum.removeListener('accountsChanged', handleAccountsChanged);
      }
    };
  }, [remoteAuthState?.loginType, remoteAuthState?.authIdentifier, disconnect]);
}
