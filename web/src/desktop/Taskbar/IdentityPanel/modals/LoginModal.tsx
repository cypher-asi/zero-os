import { useState, useEffect, useRef, useMemo, useCallback, type ReactNode } from 'react';
import { PanelLogin, Text, Button, type LoginProvider } from '@cypher-asi/zui';
import { Key, Github } from 'lucide-react';
import { useZeroIdAuth } from '../../../hooks/useZeroIdAuth';
import { useLinkedAccounts } from '../../../hooks/useLinkedAccounts';
import { ZidServerError, ZidNetworkError } from '@/client-services/identity/errors';
import { useIdentityStore, type IdentityTier } from '@/stores/identityStore';
import type { WalletType, ZidTokens } from '@/client-services/identity/types';
import '@/types/ethereum.d.ts';
import styles from './LoginModal.module.css';

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

interface LoginModalProps {
  onClose: () => void;
  onShowRegister?: () => void;
}

/**
 * Ethereum logo SVG icon
 */
function EthereumIcon(): ReactNode {
  return (
    <svg width="20" height="20" viewBox="0 0 256 417" fill="currentColor">
      <path d="M127.961 0l-2.795 9.5v275.668l2.795 2.79 127.962-75.638z" opacity="0.6" />
      <path d="M127.962 0L0 212.32l127.962 75.639V154.158z" opacity="0.45" />
      <path d="M127.961 312.187l-1.575 1.92v98.199l1.575 4.6L256 236.587z" opacity="0.8" />
      <path d="M127.962 416.905v-104.72L0 236.585z" opacity="0.45" />
    </svg>
  );
}

/**
 * Get provider icon based on provider name
 */
function getProviderIcon(providerName: string): ReactNode {
  const name = providerName.toLowerCase();

  switch (name) {
    case 'google':
      return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
          <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
          <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
          <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
        </svg>
      );

    case 'github':
      return <Github size={20} />;

    case 'twitter':
    case 'x':
      return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
        </svg>
      );

    case 'ethereum':
      return <EthereumIcon />;

    default:
      return null;
  }
}

/**
 * Capitalize first letter of a string
 */
function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

export function LoginModal({ onClose, onShowRegister }: LoginModalProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);

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

  // Handle email/password login
  const handleEmailPasswordLogin = async (emailValue: string, passwordValue: string) => {
    setIsLoading(true);
    setError(null);

    try {
      await loginWithEmail(emailValue, passwordValue);
      // Success - close modal
      onClose();
    } catch (err) {
      console.error('[LoginModal] Email login error:', err);

      // Handle specific error types
      if (err instanceof ZidServerError) {
        const reason = err.reason.toLowerCase();
        if (reason.includes('internal_error')) {
          setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
          console.error('[LoginModal] ZID server internal error - check server logs for details');
        } else if (reason.includes('invalid_credentials') || reason.includes('authentication_failed')) {
          setError('Invalid email or password. Please check your credentials and try again.');
        } else if (reason.includes('account_locked') || reason.includes('too_many_attempts')) {
          setError('Account temporarily locked due to too many failed attempts. Please try again later.');
        } else {
          setError(`Server error: ${err.reason}`);
        }
      } else if (err instanceof ZidNetworkError) {
        setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        console.error('[LoginModal] Network error:', err.message);
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
      console.error('[LoginModal] Machine key login error:', err);

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
      console.log('[LoginModal] Connected wallet address:', address);

      // Now we can set loading state - wallet is connected
      setIsLoading(true);
      setError(null);

      try {
        // Step 1: Construct the login message with current timestamp
        // Format: "Sign in to zid\nTimestamp: <unix_timestamp>\nWallet: <wallet_address>"
        const timestamp = Math.floor(Date.now() / 1000);
        const message = `Sign in to zid\nTimestamp: ${timestamp}\nWallet: ${address}`;

        console.log('[LoginModal] Signing login message...');

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

        console.log('[LoginModal] Message signed, sending login request...');

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
        console.log('[LoginModal] Wallet login successful');

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
        const odUserId = tokens.machine_id.replace(/-/g, '');
        setCurrentUser({
          id: odUserId,
          displayName: `${address.slice(0, 6)}...${address.slice(-4)}`,
          status: 'Active',
          createdAt: Date.now(),
          lastActiveAt: Date.now(),
        });

        // Create session record
        setCurrentSession({
          id: tokens.session_id.replace(/-/g, ''),
          odUserId,
          createdAt: Date.now(),
          expiresAt,
          capabilities: ['endpoint.read', 'endpoint.write'],
          loginType: 'wallet',
        });

        // Success - close modal
        onClose();
      } catch (err) {
        console.error('[LoginModal] Wallet login error:', err);

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
      label: 'Continue with Ethereum',
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
        label: `Continue with ${capitalize(cred.identifier)}`,
        onClick: async () => {
          // TODO: Implement OAuth login flow
          console.log(`[LoginModal] OAuth login with ${cred.identifier} - not yet implemented`);
          setError(`OAuth login with ${capitalize(cred.identifier)} is not yet implemented`);
        },
      });
    });

    return providers;
  }, [linkedAccountsState.credentials, handleWalletLogin]);

  // Click outside to close
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      // Only close if clicking directly on the overlay (not on modal content)
      if (event.target === overlayRef.current) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose]);

  // ESC key to close
  useEffect(() => {
    const handleEscKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscKey);
    return () => {
      document.removeEventListener('keydown', handleEscKey);
    };
  }, [onClose]);

  const displayError = error || authError;
  const isAnyLoading = isLoading || isAuthenticating;

  return (
    <div ref={overlayRef} className={styles.overlay}>
      <div ref={modalRef} className={styles.modal}>
        <div className={styles.modalContent}>
          <PanelLogin
            appName="ZERO OS"
            description="Please enter your credentials"
            emailValue={email}
            onEmailChange={setEmail}
            passwordValue={password}
            onPasswordChange={setPassword}
            onSubmit={handleEmailPasswordLogin}
            isLoading={isAnyLoading}
            image="https://i.pinimg.com/736x/7d/a9/93/7da993ea181a49912defefcc4c41c33a.jpg"
            imageHeight="200px"
            loginProviders={loginProviders}
            error={displayError}
            bottomContent={
              <>
                <Button
                  variant="ghost"
                  onClick={handleMachineKeyLogin}
                  disabled={isAnyLoading}
                  className={styles.machineKeyButton}
                >
                  <Key size={16} />
                  Login with Machine Key
                </Button>
                {onShowRegister && (
                  <div className={styles.registerLink}>
                    <Text size="sm" variant="secondary">
                      Don't have an account?
                    </Text>
                    <Button
                      variant="link"
                      onClick={onShowRegister}
                      disabled={isAnyLoading}
                      className={styles.registerButton}
                    >
                      Create Account
                    </Button>
                  </div>
                )}
              </>
            }
          />
        </div>

        {/* Error display below login panel - positioned absolutely */}
        {displayError && (
          <div className={styles.errorBox}>
            <Text variant="secondary" size="sm" style={{ color: '#ef4444' }}>
              {displayError}
            </Text>
          </div>
        )}
      </div>
    </div>
  );
}
