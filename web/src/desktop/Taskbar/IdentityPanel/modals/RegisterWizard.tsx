import { useState, useEffect, useRef, useCallback } from 'react';
import { X, Mail, Shield, User, ArrowLeft, Check } from 'lucide-react';
import { useIdentityServiceClient } from '../../../hooks/useIdentityServiceClient';
import type { RegistrationResult, OAuthProvider, WalletType, ZidTokens } from '@/client-services/identity/types';
import { ZidServerError, ZidNetworkError } from '@/client-services/identity/errors';
import { useIdentityStore, type IdentityTier } from '@/stores/identityStore';
import styles from './RegisterWizard.module.css';
// Import ethereum types for window.ethereum
import '@/types/ethereum.d.ts';

// =============================================================================
// Types
// =============================================================================

type WizardStep = 'account-type' | 'managed-method' | 'email-form' | 'complete';
type AccountType = 'managed' | 'self_sovereign' | null;

interface RegisterWizardProps {
  onClose: () => void;
  onSelfSovereignSelected?: () => void; // Callback to switch to Neural Key panel
}

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

// =============================================================================
// Component
// =============================================================================

export function RegisterWizard({ onClose, onSelfSovereignSelected }: RegisterWizardProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);

  // State
  const [step, setStep] = useState<WizardStep>('account-type');
  const [accountType, setAccountType] = useState<AccountType>(null);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Hooks
  const { client } = useIdentityServiceClient();
  const setRemoteAuthState = useIdentityStore((state) => state.setRemoteAuthState);
  const setTierStatus = useIdentityStore((state) => state.setTierStatus);
  const setCurrentUser = useIdentityStore((state) => state.setCurrentUser);
  const setCurrentSession = useIdentityStore((state) => state.setCurrentSession);

  // Handle account type selection
  const handleAccountTypeSelect = (type: AccountType) => {
    setAccountType(type);
    setError(null);

    if (type === 'self_sovereign') {
      // Redirect to Neural Key wizard
      if (onSelfSovereignSelected) {
        onSelfSovereignSelected();
      }
      onClose();
    } else if (type === 'managed') {
      setStep('managed-method');
    }
  };

  // Handle email registration
  const handleEmailRegistration = useCallback(async () => {
    if (!client) {
      setError('Service not available. Please try again.');
      return;
    }

    // Validation
    if (!email.trim()) {
      setError('Email is required');
      return;
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(email)) {
      setError('Please enter a valid email address');
      return;
    }

    if (!password) {
      setError('Password is required');
      return;
    }

    if (password.length < 12) {
      setError('Password must be at least 12 characters');
      return;
    }

    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const result: RegistrationResult = await client.registerWithEmail(
        email,
        password,
        DEFAULT_ZID_ENDPOINT
      );

      console.log('[RegisterWizard] Registration successful:', result);

      // Auto-login: Set the remote auth state with returned tokens
      const expiresAt = new Date(result.expires_at).getTime();
      setRemoteAuthState({
        serverEndpoint: DEFAULT_ZID_ENDPOINT,
        accessToken: result.access_token,
        tokenExpiresAt: expiresAt,
        refreshToken: result.refresh_token,
        scopes: [],
        sessionId: result.session_id,
        machineId: result.machine_id,
        loginType: 'email',
        authIdentifier: email, // Store the email address used for registration
      });

      // Set tier status based on registration result
      setTierStatus({
        tier: result.tier as IdentityTier,
        authMethodsCount: 1,
        canUpgrade: result.tier === 'managed',
        upgradeRequirements: result.tier === 'managed' ? ['Add second auth method'] : [],
      });

      // Create user record from identity ID
      const userId = result.identity_id.replace(/-/g, '');
      setCurrentUser({
        id: userId,
        displayName: email.split('@')[0].toUpperCase(),
        status: 'Active',
        createdAt: Date.now(),
        lastActiveAt: Date.now(),
      });

      // Create session record
      setCurrentSession({
        id: result.session_id.replace(/-/g, ''),
        userId,
        createdAt: Date.now(),
        expiresAt,
        capabilities: ['endpoint.read', 'endpoint.write'],
        loginType: 'email',
      });

      // Log warning if present
      if (result.warning) {
        console.log('[RegisterWizard] Server warning:', result.warning);
      }

      // Move to success step
      setStep('complete');
    } catch (err) {
      console.error('[RegisterWizard] Email registration error:', err);

      // Handle specific error types
      if (err instanceof ZidServerError) {
        const reason = err.reason.toLowerCase();
        if (reason.includes('internal_error')) {
          setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
          console.error('[RegisterWizard] ZID server internal error - check server logs for details');
        } else if (reason.includes('email_already_exists') || reason.includes('already registered')) {
          setError('This email is already registered. Please log in or use a different email.');
        } else if (reason.includes('invalid_email')) {
          setError('Please enter a valid email address.');
        } else if (reason.includes('weak_password') || reason.includes('password_too')) {
          setError('Password does not meet requirements. Please use a stronger password.');
        } else {
          setError(`Server error: ${err.reason}`);
        }
      } else if (err instanceof ZidNetworkError) {
        setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        console.error('[RegisterWizard] Network error:', err.message);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Registration failed. Please try again.');
      }
    } finally {
      setIsLoading(false);
    }
  }, [client, email, password, confirmPassword, setRemoteAuthState, setTierStatus, setCurrentUser, setCurrentSession]);

  // Handle OAuth registration
  const handleOAuthRegistration = useCallback(
    async (provider: OAuthProvider) => {
      if (!client) {
        setError('Service not available. Please try again.');
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        // Initiate OAuth flow - this returns an auth URL
        const { authUrl } = await client.initiateOAuth(provider, DEFAULT_ZID_ENDPOINT);

        // Open OAuth provider in new window
        // In a real implementation, you'd listen for the callback
        window.open(authUrl, '_blank', 'width=600,height=700');

        setError(
          `OAuth registration with ${provider} is pending implementation. ` +
            'Please use email registration for now.'
        );
      } catch (err) {
        console.error('[RegisterWizard] OAuth registration error:', err);

        if (err instanceof ZidServerError) {
          const reason = err.reason.toLowerCase();
          if (reason.includes('internal_error')) {
            setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
          } else {
            setError(`Server error: ${err.reason}`);
          }
        } else if (err instanceof ZidNetworkError) {
          setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        } else if (err instanceof Error) {
          setError(err.message);
        } else {
          setError('OAuth registration failed. Please try again.');
        }
      } finally {
        setIsLoading(false);
      }
    },
    [client]
  );

  // Handle wallet registration (Ethereum/MetaMask flow)
  const handleWalletRegistration = useCallback(
    async (walletType: WalletType) => {
      if (!client) {
        setError('Service not available. Please try again.');
        return;
      }

      // For Solana, show coming soon message
      if (walletType === 'solana') {
        setError(
          'Solana wallet registration is coming soon. Please use Ethereum or email registration for now.'
        );
        return;
      }

      // Check for Ethereum wallet (MetaMask)
      if (!window.ethereum) {
        setError('No Ethereum wallet detected. Please install MetaMask or another Web3 wallet.');
        return;
      }

      // IMPORTANT: Call eth_requestAccounts FIRST, before any state updates!
      // MetaMask requires this to be called synchronously in response to a user click.
      // If we set state first, the browser loses track of the "user gesture" and
      // MetaMask won't open its popup (just shows notification badge instead).
      let accounts: string[] | null = null;
      try {
        accounts = await window.ethereum.request<string[]>({
          method: 'eth_requestAccounts',
        });
      } catch (err: unknown) {
        // MetaMask errors are objects with code and message properties
        const ethError = err as { code?: number; message?: string };
        const errorCode = ethError?.code;
        const errorMessage = (ethError?.message ?? '').toLowerCase();
        
        // Handle specific MetaMask error codes
        // -32002: Request already pending
        // 4001: User rejected the request
        if (errorCode === -32002 || errorMessage.includes('already pending')) {
          setError('A wallet request is already pending. Please check MetaMask and approve or reject the pending request.');
          return;
        }
        if (errorCode === 4001 || errorMessage.includes('user rejected') || errorMessage.includes('user denied')) {
          setError('Wallet connection was rejected. Please try again.');
          return;
        }
        
        // For other errors, show the message if available
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
      console.log('[RegisterWizard] Connected wallet address:', address);

      // Now we can set loading state - wallet is connected
      setIsLoading(true);
      setError(null);

      try {

        // Step 2: Get challenge from ZID server
        const challenge = await client.initiateWalletAuth(walletType, address, DEFAULT_ZID_ENDPOINT);
        console.log('[RegisterWizard] Received challenge:', challenge.challenge_id);

        // Step 3: Sign the challenge message with MetaMask (EIP-191 personal_sign)
        // Server returns "message_to_sign" field
        const messageToSign = challenge.message_to_sign ?? challenge.message;
        if (!messageToSign) {
          throw new Error('No message to sign in challenge response');
        }
        const signature = await window.ethereum.request<string>({
          method: 'personal_sign',
          params: [messageToSign, address],
        });

        if (!signature) {
          throw new Error('Failed to sign message. Please try again.');
        }

        // Strip 0x prefix from signature - ZID server expects raw hex
        const signatureHex = signature.startsWith('0x') ? signature.slice(2) : signature;

        console.log('[RegisterWizard] Message signed, verifying...');

        // Step 4: Verify the signature with ZID server
        const tokens: ZidTokens = await client.verifyWalletAuth(
          challenge.challenge_id,
          walletType,
          address,
          signatureHex,
          DEFAULT_ZID_ENDPOINT
        );

        console.log('[RegisterWizard] Wallet registration successful');

        // Step 5: Update identity store with the returned tokens
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
          authIdentifier: address, // Store the wallet address
        });

        // Set tier status (wallet registrations are managed tier)
        setTierStatus({
          tier: 'managed' as IdentityTier,
          authMethodsCount: 1,
          canUpgrade: true,
          upgradeRequirements: ['Add second auth method'],
        });

        // Create user record from session/machine ID
        // For wallet auth, we use the machine_id as the user identifier
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

        // Log warning if present
        if (tokens.warning) {
          console.log('[RegisterWizard] Server warning:', tokens.warning);
        }

        // Move to success step
        setStep('complete');
      } catch (err) {
        console.error('[RegisterWizard] Wallet registration error:', err);

        // Handle specific error types
        if (err instanceof ZidServerError) {
          // Extract meaningful info from server errors
          const reason = err.reason.toLowerCase();
          if (reason.includes('internal_error')) {
            setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
            console.error('[RegisterWizard] ZID server internal error - check server logs for details');
          } else if (reason.includes('rate_limit') || reason.includes('too_many')) {
            setError('Too many requests. Please wait a moment and try again.');
          } else if (reason.includes('maintenance') || reason.includes('unavailable')) {
            setError('The ZERO ID server is temporarily unavailable. Please try again later.');
          } else {
            setError(`Server error: ${err.reason}`);
          }
        } else if (err instanceof ZidNetworkError) {
          setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
          console.error('[RegisterWizard] Network error:', err.message);
        } else if (err instanceof Error) {
          const msg = err.message.toLowerCase();
          if (msg.includes('user rejected') || msg.includes('user denied')) {
            // This catches signature rejection (connection rejection is handled earlier)
            setError('Signature request was rejected. Please try again and sign the message to verify wallet ownership.');
          } else if (msg.includes('already processing')) {
            setError('A wallet request is already pending. Please check MetaMask.');
          } else if (msg.includes('already linked') || msg.includes('already registered')) {
            // Wallet is already registered
            setError('This wallet is already linked to an existing identity. If this is your wallet, you can log in with your existing credentials.');
          } else {
            setError(err.message);
          }
        } else {
          setError('Wallet registration failed. Please try again.');
        }
      } finally {
        setIsLoading(false);
      }
    },
    [client, setRemoteAuthState, setTierStatus, setCurrentUser, setCurrentSession]
  );

  // Click outside to close
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (event.target === overlayRef.current) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  // ESC key to close
  useEffect(() => {
    const handleEscKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscKey);
    return () => document.removeEventListener('keydown', handleEscKey);
  }, [onClose]);

  // Navigation helpers
  const goBack = () => {
    setError(null);
    if (step === 'email-form') {
      setStep('managed-method');
    } else if (step === 'managed-method') {
      setStep('account-type');
      setAccountType(null);
    }
  };

  // Step indicator
  const getStepNumber = () => {
    switch (step) {
      case 'account-type':
        return 1;
      case 'managed-method':
      case 'email-form':
        return 2;
      case 'complete':
        return 3;
      default:
        return 1;
    }
  };

  // Get title based on step
  const getTitle = () => {
    switch (step) {
      case 'account-type':
        return 'Create Account';
      case 'managed-method':
        return 'Choose Sign-Up Method';
      case 'email-form':
        return 'Email Registration';
      case 'complete':
        return 'Welcome!';
      default:
        return 'Create Account';
    }
  };

  return (
    <div ref={overlayRef} className={styles.overlay}>
      <div ref={modalRef} className={styles.modal}>
        {/* Header */}
        <div className={styles.header}>
          <h2 className={styles.headerTitle}>{getTitle()}</h2>
          <button className={styles.closeButton} onClick={onClose} aria-label="Close">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className={styles.content}>
          {/* Step indicator */}
          {step !== 'complete' && (
            <div className={styles.stepIndicator}>
              {[1, 2, 3].map((s) => (
                <div
                  key={s}
                  className={`${styles.stepDot} ${s <= getStepNumber() ? styles.active : ''}`}
                />
              ))}
            </div>
          )}

          {/* Step 1: Account Type Selection */}
          {step === 'account-type' && (
            <div className={styles.accountTypeCards}>
              <div
                className={`${styles.accountTypeCard} ${accountType === 'managed' ? styles.selected : ''}`}
                onClick={() => handleAccountTypeSelect('managed')}
              >
                <div className={styles.cardHeader}>
                  <div className={styles.cardIcon}>
                    <User size={20} />
                  </div>
                  <span className={styles.cardTitle}>Managed Identity</span>
                </div>
                <p className={styles.cardDescription}>
                  Quick and easy setup. Sign up with email, OAuth, or wallet. Your identity is
                  protected by ZERO ID servers. Great for getting started.
                </p>
              </div>

              <div
                className={`${styles.accountTypeCard} ${accountType === 'self_sovereign' ? styles.selected : ''}`}
                onClick={() => handleAccountTypeSelect('self_sovereign')}
              >
                <div className={styles.cardHeader}>
                  <div className={styles.cardIcon}>
                    <Shield size={20} />
                  </div>
                  <span className={styles.cardTitle}>Self-Sovereign Identity</span>
                </div>
                <p className={styles.cardDescription}>
                  Full control over your identity. Generate a Neural Key that only you control. Your
                  keys never leave your device. Best for security-conscious users.
                </p>
              </div>
            </div>
          )}

          {/* Step 2a: Managed Registration Method */}
          {step === 'managed-method' && (
            <>
              <button className={styles.backLink} onClick={goBack}>
                <ArrowLeft size={16} />
                Back
              </button>

              <div className={styles.methodList}>
                <button
                  className={styles.methodButton}
                  onClick={() => setStep('email-form')}
                  disabled={isLoading}
                >
                  <div className={styles.methodIcon}>
                    <Mail size={20} />
                  </div>
                  Sign up with Email
                </button>

                <div className={styles.divider}>
                  <div className={styles.dividerLine} />
                  <span className={styles.dividerText}>or continue with</span>
                  <div className={styles.dividerLine} />
                </div>

                <button
                  className={styles.methodButton}
                  onClick={() => handleOAuthRegistration('google')}
                  disabled={isLoading}
                >
                  <div className={styles.methodIcon}>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
                      <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
                      <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
                      <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
                    </svg>
                  </div>
                  Continue with Google
                </button>

                <button
                  className={styles.methodButton}
                  onClick={() => handleOAuthRegistration('x')}
                  disabled={isLoading}
                >
                  <div className={styles.methodIcon}>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
                    </svg>
                  </div>
                  Continue with X
                </button>

                <div className={styles.divider}>
                  <div className={styles.dividerLine} />
                  <span className={styles.dividerText}>or connect wallet</span>
                  <div className={styles.dividerLine} />
                </div>

                <button
                  className={styles.methodButton}
                  onClick={() => handleWalletRegistration('ethereum')}
                  disabled={isLoading}
                >
                  <div className={styles.methodIcon}>
                    <svg width="20" height="20" viewBox="0 0 256 417" fill="currentColor">
                      <path d="M127.961 0l-2.795 9.5v275.668l2.795 2.79 127.962-75.638z" opacity="0.6" />
                      <path d="M127.962 0L0 212.32l127.962 75.639V154.158z" opacity="0.45" />
                      <path d="M127.961 312.187l-1.575 1.92v98.199l1.575 4.6L256 236.587z" opacity="0.8" />
                      <path d="M127.962 416.905v-104.72L0 236.585z" opacity="0.45" />
                    </svg>
                  </div>
                  Continue with Ethereum
                </button>

                <button
                  className={styles.methodButton}
                  onClick={() => handleWalletRegistration('solana')}
                  disabled={isLoading}
                >
                  <div className={styles.methodIcon}>
                    <svg width="20" height="20" viewBox="0 0 397.7 311.7" fill="currentColor">
                      <path d="M64.6 237.9c2.4-2.4 5.7-3.8 9.2-3.8h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1l62.7-62.7z" />
                      <path d="M64.6 3.8C67.1 1.4 70.4 0 73.8 0h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1L64.6 3.8z" />
                      <path d="M333.1 120.1c-2.4-2.4-5.7-3.8-9.2-3.8H6.5c-5.8 0-8.7 7-4.6 11.1l62.7 62.7c2.4 2.4 5.7 3.8 9.2 3.8h317.4c5.8 0 8.7-7 4.6-11.1l-62.7-62.7z" />
                    </svg>
                  </div>
                  Continue with Solana
                </button>
              </div>
            </>
          )}

          {/* Step 2b: Email Form */}
          {step === 'email-form' && (
            <>
              <button className={styles.backLink} onClick={goBack}>
                <ArrowLeft size={16} />
                Back
              </button>

              <div className={styles.emailForm}>
                <div className={styles.inputGroup}>
                  <label className={styles.inputLabel}>Email</label>
                  <input
                    type="email"
                    className={styles.input}
                    placeholder="you@example.com"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    disabled={isLoading}
                    autoComplete="email"
                  />
                </div>

                <div className={styles.inputGroup}>
                  <label className={styles.inputLabel}>Password</label>
                  <input
                    type="password"
                    className={styles.input}
                    placeholder="Create a strong password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    disabled={isLoading}
                    autoComplete="new-password"
                  />
                  <span className={styles.passwordHint}>At least 12 characters</span>
                </div>

                <div className={styles.inputGroup}>
                  <label className={styles.inputLabel}>Confirm Password</label>
                  <input
                    type="password"
                    className={styles.input}
                    placeholder="Confirm your password"
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    disabled={isLoading}
                    autoComplete="new-password"
                  />
                </div>
              </div>
            </>
          )}

          {/* Step 3: Complete */}
          {step === 'complete' && (
            <div className={styles.successBox}>
              <div className={styles.successIcon}>
                <Check size={32} />
              </div>
              <h3 className={styles.successTitle}>Welcome to ZERO OS!</h3>
              <p className={styles.successDescription}>
                Your account has been created and you're now signed in. Consider adding more
                authentication methods for enhanced security and to unlock self-sovereign identity
                features.
              </p>
            </div>
          )}

          {/* Error display */}
          {error && <div className={styles.errorBox}>{error}</div>}
        </div>

        {/* Footer */}
        {step !== 'account-type' && (
          <div
            className={`${styles.footer} ${step === 'complete' ? styles.footerSingle : ''}`}
          >
            {step === 'email-form' && (
              <>
                <button
                  className={`${styles.button} ${styles.buttonSecondary}`}
                  onClick={goBack}
                  disabled={isLoading}
                >
                  Back
                </button>
                <button
                  className={`${styles.button} ${styles.buttonPrimary}`}
                  onClick={handleEmailRegistration}
                  disabled={isLoading}
                >
                  {isLoading ? (
                    <span className={styles.loadingButton}>
                      <span className={styles.spinner} />
                      Creating Account...
                    </span>
                  ) : (
                    'Create Account'
                  )}
                </button>
              </>
            )}

            {step === 'complete' && (
              <button
                className={`${styles.button} ${styles.buttonPrimary}`}
                onClick={onClose}
              >
                Get Started
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
