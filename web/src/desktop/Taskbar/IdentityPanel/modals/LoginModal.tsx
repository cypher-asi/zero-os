import { useState, useEffect, useCallback, useMemo } from 'react';
import { Button, Input, Text, type LoginProvider } from '@cypher-asi/zui';
import { Key } from 'lucide-react';
import { getProviderIcon, capitalize } from './shared/icons';
import { useZeroIdAuth } from '../../../hooks/useZeroIdAuth';
import { useLinkedAccounts } from '../../../hooks/useLinkedAccounts';
import { ZidServerError, ZidNetworkError } from '@/client-services/identity/errors';
import { useIdentityStore, type IdentityTier } from '@/stores/identityStore';
import type { WalletType, ZidTokens } from '@/client-services/identity/types';
import '@/types/ethereum.d.ts';
import styles from './LoginModal.module.css';

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

// =============================================================================
// LoginContent - Embeddable login form content
// =============================================================================

interface LoginContentProps {
  onClose: () => void;
  /** Callback when loading state changes (for parent to track) */
  onLoadingChange?: (isLoading: boolean) => void;
}

/**
 * LoginContent - The actual login form content without overlay
 *
 * Designed to be embedded in AuthPanel. Contains:
 * - Email/password form
 * - Login providers (wallets, OAuth)
 * - Machine key login button
 */
export function LoginContent({ onClose, onLoadingChange }: LoginContentProps) {
  // Auth hooks
  const {
    loginWithEmail,
    loginWithMachineKey,
    isAuthenticating,
    error: authError,
  } = useZeroIdAuth();

  const { state: linkedAccountsState } = useLinkedAccounts();
  
  // Identity store setters for wallet login
  const setRemoteAuthState = useIdentityStore((state) => state.setRemoteAuthState);
  const setTierStatus = useIdentityStore((state) => state.setTierStatus);
  const setCurrentUser = useIdentityStore((state) => state.setCurrentUser);
  const setCurrentSession = useIdentityStore((state) => state.setCurrentSession);

  // Form state
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isAnyLoading = isLoading || isAuthenticating;

  // Notify parent of loading state changes
  useEffect(() => {
    onLoadingChange?.(isAnyLoading);
  }, [isAnyLoading, onLoadingChange]);

  // Handle email/password login
  const handleEmailPasswordLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!email.trim() || !password) {
      setError('Please enter email and password');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await loginWithEmail(email, password);
      // Success - close modal
      onClose();
    } catch (err) {
      console.error('[LoginContent] Email login error:', err);

      // Handle specific error types
      if (err instanceof ZidServerError) {
        const reason = err.reason.toLowerCase();
        if (reason.includes('internal_error')) {
          setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
          console.error('[LoginContent] ZID server internal error - check server logs for details');
        } else if (reason.includes('invalid_credentials') || reason.includes('authentication_failed')) {
          setError('Invalid email or password. Please check your credentials and try again.');
        } else if (reason.includes('account_locked') || reason.includes('too_many_attempts')) {
          setError('Account temporarily locked due to too many failed attempts. Please try again later.');
        } else {
          setError(`Server error: ${err.reason}`);
        }
      } else if (err instanceof ZidNetworkError) {
        setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        console.error('[LoginContent] Network error:', err.message);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Login failed. Please try again.');
      }
    } finally {
      setIsLoading(false);
    }
  };

  // Handle machine key login
  const handleMachineKeyLogin = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      await loginWithMachineKey();
      // Success - close modal
      onClose();
    } catch (err) {
      console.error('[LoginContent] Machine key login error:', err);

      // Handle specific error types
      if (err instanceof ZidServerError) {
        const reason = err.reason.toLowerCase();
        if (reason.includes('internal_error')) {
          setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
        } else if (reason.includes('machine_not_registered') || reason.includes('not_found')) {
          setError('This machine is not registered. Please enroll this machine first.');
        } else {
          setError(`Server error: ${err.reason}`);
        }
      } else if (err instanceof ZidNetworkError) {
        setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Machine key login failed. Please try again.');
      }
    } finally {
      setIsLoading(false);
    }
  }, [loginWithMachineKey, onClose]);

  // Handle wallet login (Ethereum/MetaMask flow)
  // Uses direct POST to /v1/auth/login/wallet with client-constructed message
  const handleWalletLogin = useCallback(
    async (_walletType: WalletType) => {
      // Check for Ethereum wallet (MetaMask)
      if (!window.ethereum) {
        setError('No Ethereum wallet detected. Please install MetaMask or another Web3 wallet.');
        return;
      }

      // IMPORTANT: Call eth_requestAccounts FIRST, before any state updates!
      // MetaMask requires this to be called synchronously in response to a user click.
      let accounts: string[] | null = null;
      try {
        accounts = await window.ethereum.request<string[]>({
          method: 'eth_requestAccounts',
        });
      } catch (err: unknown) {
        const ethError = err as { code?: number; message?: string };
        const errorCode = ethError?.code;
        const errorMessage = (ethError?.message ?? '').toLowerCase();

        if (errorCode === -32002 || errorMessage.includes('already pending')) {
          setError('A wallet request is already pending. Please check MetaMask and approve or reject the pending request.');
          return;
        }
        if (errorCode === 4001 || errorMessage.includes('user rejected') || errorMessage.includes('user denied')) {
          setError('Wallet connection was rejected. Please try again.');
          return;
        }
        if (errorMessage) {
          setError(`Wallet error: ${ethError.message}`);
          return;
        }
        throw err;
      }

      if (!accounts || accounts.length === 0) {
        setError('No accounts returned from wallet. Please try again.');
        return;
      }

      const address = accounts[0].toLowerCase();
      console.log('[LoginContent] Connected wallet address:', address);

      // Now we can set loading state - wallet is connected
      setIsLoading(true);
      setError(null);

      try {
        // Step 1: Construct the login message with current timestamp
        // Format: "Sign in to zid\nTimestamp: <unix_timestamp>\nWallet: <wallet_address>"
        const timestamp = Math.floor(Date.now() / 1000);
        const message = `Sign in to zid\nTimestamp: ${timestamp}\nWallet: ${address}`;

        console.log('[LoginContent] Signing login message...');

        // Step 2: Sign the message with MetaMask (EIP-191 personal_sign)
        const signature = await window.ethereum.request<string>({
          method: 'personal_sign',
          params: [message, address],
        });

        if (!signature) {
          throw new Error('Failed to sign message. Please try again.');
        }

        // Strip 0x prefix from signature - ZID server expects raw hex
        const signatureHex = signature.startsWith('0x') ? signature.slice(2) : signature;

        console.log('[LoginContent] Message signed, sending login request...');

        // Step 3: POST to /v1/auth/login/wallet
        const response = await fetch(`${DEFAULT_ZID_ENDPOINT}/v1/auth/login/wallet`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            wallet_address: address,
            signature: signatureHex,
            message: message,
          }),
        });

        if (!response.ok) {
          const errorBody = await response.json().catch(() => ({}));
          const errorMessage = errorBody?.error?.message || errorBody?.error || errorBody?.message;
          
          if (response.status === 401) {
            throw new Error('Invalid signature. Please try again.');
          } else if (response.status === 404) {
            throw new Error('This wallet is not registered. Please create an account first.');
          } else if (response.status === 400) {
            throw new Error(errorMessage || 'Invalid request. The timestamp may have expired - please try again.');
          } else {
            throw new Error(errorMessage || `Login failed with status ${response.status}`);
          }
        }

        const tokens: ZidTokens = await response.json();
        console.log('[LoginContent] Wallet login successful');

        // Step 4: Update identity store with the returned tokens
        const expiresAt = new Date(tokens.expires_at).getTime();
        setRemoteAuthState({
          serverEndpoint: DEFAULT_ZID_ENDPOINT,
          accessToken: tokens.access_token,
          tokenExpiresAt: expiresAt,
          refreshToken: tokens.refresh_token,
          scopes: [],
          sessionId: tokens.session_id,
          machineId: tokens.machine_id,
          loginType: 'wallet',
          authIdentifier: address,
        });

        // Set tier status (wallet logins are managed tier)
        setTierStatus({
          tier: 'managed' as IdentityTier,
          authMethodsCount: 1,
          canUpgrade: true,
          upgradeRequirements: ['Add second auth method'],
        });

        // Create user record
        const userId = tokens.machine_id.replace(/-/g, '');
        setCurrentUser({
          id: userId,
          displayName: `${address.slice(0, 6)}...${address.slice(-4)}`,
          status: 'Active',
          createdAt: Date.now(),
          lastActiveAt: Date.now(),
        });

        // Create session record
        setCurrentSession({
          id: tokens.session_id.replace(/-/g, ''),
          userId,
          createdAt: Date.now(),
          expiresAt,
          capabilities: ['endpoint.read', 'endpoint.write'],
          loginType: 'wallet',
        });

        // Success - close modal
        onClose();
      } catch (err) {
        console.error('[LoginContent] Wallet login error:', err);

        if (err instanceof TypeError && err.message.includes('fetch')) {
          setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        } else if (err instanceof Error) {
          const msg = err.message.toLowerCase();
          if (msg.includes('user rejected') || msg.includes('user denied')) {
            setError('Signature request was rejected. Please try again and sign the message to verify wallet ownership.');
          } else if (msg.includes('already processing')) {
            setError('A wallet request is already pending. Please check MetaMask.');
          } else {
            setError(err.message);
          }
        } else {
          setError('Wallet login failed. Please try again.');
        }
      } finally {
        setIsLoading(false);
      }
    },
    [onClose, setRemoteAuthState, setTierStatus, setCurrentUser, setCurrentSession]
  );

  // Build login providers (wallet + OAuth from linked accounts)
  const loginProviders: LoginProvider[] = useMemo(() => {
    const providers: LoginProvider[] = [];

    // Always show Ethereum wallet login option
    providers.push({
      id: 'ethereum',
      icon: getProviderIcon('ethereum'),
      label: 'Ethereum',
      onClick: async () => {
        await handleWalletLogin('ethereum');
      },
    });

    // Add OAuth providers from linked accounts
    const oauthCredentials = linkedAccountsState.credentials.filter((c) => c.type === 'oauth');
    oauthCredentials.forEach((cred) => {
      providers.push({
        id: cred.identifier,
        icon: getProviderIcon(cred.identifier),
        label: capitalize(cred.identifier),
        onClick: async () => {
          // TODO: Implement OAuth login flow
          console.log(`[LoginContent] OAuth login with ${cred.identifier} - not yet implemented`);
          setError(`OAuth login with ${capitalize(cred.identifier)} is not yet implemented`);
        },
      });
    });

    return providers;
  }, [linkedAccountsState.credentials, handleWalletLogin]);

  const displayError = error || authError;

  return (
    <div className={styles.loginContent}>
      {/* Header */}
      <div className={styles.header}>
        <Text size="lg" className={styles.title}>ZERO OS</Text>
        <Text size="sm" variant="secondary">Please enter your credentials</Text>
      </div>

      {/* Email/Password Form */}
      <form onSubmit={handleEmailPasswordLogin} className={styles.form}>
        <Input
          type="email"
          placeholder="E-mail"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={isAnyLoading}
          autoComplete="email"
        />
        <Input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={isAnyLoading}
          autoComplete="current-password"
        />
        <Button
          type="submit"
          variant="secondary"
          disabled={isAnyLoading}
          className={styles.submitButton}
        >
          {isAnyLoading ? 'Signing in...' : 'Sign In'}
        </Button>
      </form>

      {/* Divider */}
      <div className={styles.divider}>
        <div className={styles.dividerLine} />
        <Text size="xs" variant="secondary">or continue with</Text>
        <div className={styles.dividerLine} />
      </div>

      {/* Login Providers */}
      <div className={styles.providers}>
        {loginProviders.map((provider) => (
          <Button
            key={provider.id}
            variant="secondary"
            onClick={provider.onClick}
            disabled={isAnyLoading}
            className={styles.providerButton}
          >
            {provider.icon}
            {provider.label}
          </Button>
        ))}
      </div>

      {/* Machine Key Login */}
      <Button
        variant="ghost"
        onClick={handleMachineKeyLogin}
        disabled={isAnyLoading}
        className={styles.machineKeyButton}
      >
        <Key size={16} />
        Machine Key
      </Button>

      {/* Error Display */}
      {displayError && (
        <div className={styles.errorBox}>
          <Text variant="secondary" size="sm" style={{ color: '#ef4444' }}>
            {displayError}
          </Text>
        </div>
      )}
    </div>
  );
}
