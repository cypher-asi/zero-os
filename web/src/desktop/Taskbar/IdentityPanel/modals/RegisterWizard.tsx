import { useState, useEffect, useCallback, useMemo } from 'react';
import { Button, Input, Text, ItemDetailed } from '@cypher-asi/zui';
import { Mail, Shield, User, Check, ChevronLeft } from 'lucide-react';
import { GoogleIcon, XIcon, EthereumIcon, SolanaIcon } from './shared/icons';
import { useIdentityServiceClient } from '../../../hooks/useIdentityServiceClient';
import type { RegistrationResult, OAuthProvider, WalletType, ZidTokens } from '@/client-services/identity/types';
import { ZidServerError, ZidNetworkError } from '@/client-services/identity/errors';
import { useIdentityStore, type IdentityTier } from '@/stores/identityStore';
import styles from './RegisterWizard.module.css';
import '@/types/ethereum.d.ts';

// =============================================================================
// Types
// =============================================================================

type AccountType = 'managed' | 'self_sovereign' | null;
type RegistrationStep = 'account-type' | 'method' | 'email-form' | 'complete';

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

// =============================================================================
// RegisterContent - Embeddable registration form content
// =============================================================================

interface RegisterContentProps {
  onClose: () => void;
  /** Callback when self-sovereign identity is selected */
  onSelfSovereignSelected?: () => void;
  /** Callback when loading state changes (for parent to track) */
  onLoadingChange?: (isLoading: boolean) => void;
}

/**
 * RegisterContent - Registration wizard content without PanelWizard
 *
 * Designed to be embedded in AuthPanel with consistent styling.
 */
export function RegisterContent({ onClose, onSelfSovereignSelected, onLoadingChange }: RegisterContentProps) {
  // State
  const [currentStep, setCurrentStep] = useState<RegistrationStep>('account-type');
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

  // Notify parent of loading state changes
  useEffect(() => {
    onLoadingChange?.(isLoading);
  }, [isLoading, onLoadingChange]);

  // Handle account type selection
  const handleAccountTypeSelect = useCallback((type: AccountType) => {
    setAccountType(type);
    setError(null);

    if (type === 'self_sovereign') {
      if (onSelfSovereignSelected) {
        onSelfSovereignSelected();
      }
      onClose();
    } else if (type === 'managed') {
      setCurrentStep('method');
    }
  }, [onClose, onSelfSovereignSelected]);

  // Handle back navigation
  const handleBack = useCallback(() => {
    setError(null);
    if (currentStep === 'method') {
      setCurrentStep('account-type');
      setAccountType(null);
    } else if (currentStep === 'email-form') {
      setCurrentStep('method');
    }
  }, [currentStep]);

  // Handle email registration
  const handleEmailRegistration = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    
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

      console.log('[RegisterContent] Registration successful:', result);

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
        authIdentifier: email,
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

      if (result.warning) {
        console.log('[RegisterContent] Server warning:', result.warning);
      }

      setCurrentStep('complete');
    } catch (err) {
      console.error('[RegisterContent] Email registration error:', err);

      if (err instanceof ZidServerError) {
        const reason = err.reason.toLowerCase();
        if (reason.includes('internal_error')) {
          setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
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
        const { authUrl } = await client.initiateOAuth(provider, DEFAULT_ZID_ENDPOINT);
        window.open(authUrl, '_blank', 'width=600,height=700');
        setError(
          `OAuth registration with ${provider} is pending implementation. ` +
            'Please use email registration for now.'
        );
      } catch (err) {
        console.error('[RegisterContent] OAuth registration error:', err);

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

  // Handle wallet registration
  const handleWalletRegistration = useCallback(
    async (walletType: WalletType) => {
      if (!client) {
        setError('Service not available. Please try again.');
        return;
      }

      if (walletType === 'solana') {
        setError('Solana wallet registration is coming soon. Please use Ethereum or email registration for now.');
        return;
      }

      if (!window.ethereum) {
        setError('No Ethereum wallet detected. Please install MetaMask or another Web3 wallet.');
        return;
      }

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
      console.log('[RegisterContent] Connected wallet address:', address);

      setIsLoading(true);
      setError(null);

      try {
        const challenge = await client.initiateWalletAuth(walletType, address, DEFAULT_ZID_ENDPOINT);
        console.log('[RegisterContent] Received challenge:', challenge.challenge_id);

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

        const signatureHex = signature.startsWith('0x') ? signature.slice(2) : signature;
        console.log('[RegisterContent] Message signed, verifying...');

        const tokens: ZidTokens = await client.verifyWalletAuth(
          challenge.challenge_id,
          walletType,
          address,
          signatureHex,
          DEFAULT_ZID_ENDPOINT
        );

        console.log('[RegisterContent] Wallet registration successful');

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

        setTierStatus({
          tier: 'managed' as IdentityTier,
          authMethodsCount: 1,
          canUpgrade: true,
          upgradeRequirements: ['Add second auth method'],
        });

        const userId = tokens.machine_id.replace(/-/g, '');
        setCurrentUser({
          id: userId,
          displayName: `${address.slice(0, 6)}...${address.slice(-4)}`,
          status: 'Active',
          createdAt: Date.now(),
          lastActiveAt: Date.now(),
        });

        setCurrentSession({
          id: tokens.session_id.replace(/-/g, ''),
          userId,
          createdAt: Date.now(),
          expiresAt,
          capabilities: ['endpoint.read', 'endpoint.write'],
          loginType: 'wallet',
        });

        if (tokens.warning) {
          console.log('[RegisterContent] Server warning:', tokens.warning);
        }

        setCurrentStep('complete');
      } catch (err) {
        console.error('[RegisterContent] Wallet registration error:', err);

        if (err instanceof ZidServerError) {
          const reason = err.reason.toLowerCase();
          if (reason.includes('internal_error')) {
            setError('The ZERO ID server encountered an error. Please ensure the ZID server is running and try again.');
          } else if (reason.includes('rate_limit') || reason.includes('too_many')) {
            setError('Too many requests. Please wait a moment and try again.');
          } else if (reason.includes('maintenance') || reason.includes('unavailable')) {
            setError('The ZERO ID server is temporarily unavailable. Please try again later.');
          } else {
            setError(`Server error: ${err.reason}`);
          }
        } else if (err instanceof ZidNetworkError) {
          setError('Unable to reach the ZERO ID server. Please check your connection and ensure the server is running.');
        } else if (err instanceof Error) {
          const msg = err.message.toLowerCase();
          if (msg.includes('user rejected') || msg.includes('user denied')) {
            setError('Signature request was rejected. Please try again and sign the message to verify wallet ownership.');
          } else if (msg.includes('already processing')) {
            setError('A wallet request is already pending. Please check MetaMask.');
          } else if (msg.includes('already linked') || msg.includes('already registered')) {
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

  // Email form validation
  const isEmailFormValid = useMemo(() => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return (
      email.trim() !== '' &&
      emailRegex.test(email) &&
      password.length >= 12 &&
      password === confirmPassword
    );
  }, [email, password, confirmPassword]);

  // Render current step
  const renderStep = () => {
    switch (currentStep) {
      case 'account-type':
        return (
          <div className={styles.stepContent}>
            <div className={styles.header}>
              <Text size="lg" className={styles.title}>Create Account</Text>
              <Text size="sm" variant="secondary">Choose your identity type</Text>
            </div>

            <div className={styles.optionsList}>
              <ItemDetailed
                id="managed"
                icon={<User size={20} />}
                label="Managed Identity"
                description="Sign up with email, OAuth, or wallet"
                selected={accountType === 'managed'}
                onClick={() => handleAccountTypeSelect('managed')}
              />

              <ItemDetailed
                id="self-sovereign"
                icon={<Shield size={20} />}
                label="Self-Sovereign Identity"
                description="Full control with your own Neural Key"
                selected={accountType === 'self_sovereign'}
                onClick={() => handleAccountTypeSelect('self_sovereign')}
              />
            </div>
          </div>
        );

      case 'method':
        return (
          <div className={styles.stepContent}>
            <div className={styles.headerWithBack}>
              <button type="button" className={styles.backButton} onClick={handleBack} disabled={isLoading}>
                <ChevronLeft size={20} />
              </button>
              <div className={styles.headerText}>
                <Text size="lg" className={styles.title}>Sign Up Method</Text>
                <Text size="sm" variant="secondary">Choose how to create your account</Text>
              </div>
            </div>

            <div className={styles.methodList}>
              <Button
                variant="secondary"
                onClick={() => setCurrentStep('email-form')}
                disabled={isLoading}
                className={styles.methodButton}
              >
                <Mail size={20} />
                Sign up with Email
              </Button>

              <div className={styles.divider}>
                <div className={styles.dividerLine} />
                <Text size="xs" variant="secondary">or continue with</Text>
                <div className={styles.dividerLine} />
              </div>

              <Button
                variant="secondary"
                disabled
                className={styles.methodButtonUnsupported}
                title="Google sign-in coming soon"
              >
                <GoogleIcon />
                Google
                <span className={styles.comingSoonLabel}>Soon</span>
              </Button>

              <Button
                variant="secondary"
                disabled
                className={styles.methodButtonUnsupported}
                title="X sign-in coming soon"
              >
                <XIcon />
                X
                <span className={styles.comingSoonLabel}>Soon</span>
              </Button>

              <div className={styles.divider}>
                <div className={styles.dividerLine} />
                <Text size="xs" variant="secondary">or connect wallet</Text>
                <div className={styles.dividerLine} />
              </div>

              <Button
                variant="secondary"
                onClick={() => handleWalletRegistration('ethereum')}
                disabled={isLoading}
                className={styles.methodButton}
              >
                <EthereumIcon />
                Ethereum
              </Button>

              <Button
                variant="secondary"
                onClick={() => handleWalletRegistration('solana')}
                disabled={isLoading}
                className={styles.methodButton}
              >
                <SolanaIcon />
                Solana
              </Button>
            </div>
          </div>
        );

      case 'email-form':
        return (
          <div className={styles.stepContent}>
            <div className={styles.headerWithBack}>
              <button type="button" className={styles.backButton} onClick={handleBack} disabled={isLoading}>
                <ChevronLeft size={20} />
              </button>
              <div className={styles.headerText}>
                <Text size="lg" className={styles.title}>Email Registration</Text>
                <Text size="sm" variant="secondary">Create your account with email</Text>
              </div>
            </div>

            <form onSubmit={handleEmailRegistration} className={styles.form}>
              <Input
                type="email"
                placeholder="E-mail"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={isLoading}
                autoComplete="email"
              />
              <Input
                type="password"
                placeholder="Password (min 12 characters)"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                disabled={isLoading}
                autoComplete="new-password"
              />
              <Input
                type="password"
                placeholder="Confirm Password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                disabled={isLoading}
                autoComplete="new-password"
              />
              <Button
                type="submit"
                variant="secondary"
                disabled={isLoading || !isEmailFormValid}
                className={styles.submitButton}
              >
                {isLoading ? 'Creating Account...' : 'Create Account'}
              </Button>
            </form>
          </div>
        );

      case 'complete':
        return (
          <div className={styles.stepContent}>
            <div className={styles.successBox}>
              <div className={styles.successIcon}>
                <Check size={32} />
              </div>
              <Text size="lg" className={styles.successTitle}>
                Welcome to ZERO OS!
              </Text>
              <Text size="sm" variant="secondary" className={styles.successDescription}>
                Your account has been created and you're now signed in. Consider adding more
                authentication methods for enhanced security.
              </Text>
              <Button variant="secondary" onClick={onClose} className={styles.getStartedButton}>
                Begin
              </Button>
            </div>
          </div>
        );
    }
  };

  return (
    <div className={styles.registerContent}>
      {renderStep()}
      
      {error && (
        <div className={styles.errorBox}>
          <Text variant="secondary" size="sm" style={{ color: '#ef4444' }}>
            {error}
          </Text>
        </div>
      )}
    </div>
  );
}
