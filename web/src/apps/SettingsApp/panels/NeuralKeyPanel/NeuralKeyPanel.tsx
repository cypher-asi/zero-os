import { useState, useCallback, useMemo } from 'react';
import {
  GroupCollapsible,
  Button,
  Card,
  CardItem,
  Text,
  Label,
  Input,
  PanelWizard,
  type WizardStep,
} from '@cypher-asi/zui';
import {
  Brain,
  AlertTriangle,
  Loader,
  Eye,
  EyeOff,
  ShieldCheck,
  RefreshCw,
  Laptop,
} from 'lucide-react';
import { useNeuralKey } from '@desktop/hooks/useNeuralKey';
import { useCopyToClipboard } from '@desktop/hooks/useCopyToClipboard';
import { useMachineKeys } from '@desktop/hooks/useMachineKeys';
import { useZeroIdAuth } from '@desktop/hooks/useZeroIdAuth';
import { NeuralKeyStatus } from './NeuralKeyStatus';
import { ShardDisplay } from './ShardDisplay';
import styles from './NeuralKeyPanel.module.css';

/** Minimum password length */
const MIN_PASSWORD_LENGTH = 12;

/** Default ZID endpoint for enrollment */
const DEFAULT_ZID_ENDPOINT = 'http://127.0.0.1:9999';

/** Step state machine */
type StepState = 'idle' | 'loading' | 'success' | 'error';

/**
 * Neural Key Panel
 *
 * States:
 * 1. Not Set - Show 5-step wizard (intro, password, machine-key, verify, backup)
 * 2. Active - Show fingerprint and created date
 */
export function NeuralKeyPanel() {
  const { state, generateNeuralKey, confirmShardsSaved } = useNeuralKey();
  const { copy, isCopied } = useCopyToClipboard();
  const { state: machineKeysState, createMachineKeyAndEnroll } = useMachineKeys();
  const {
    remoteAuthState,
    isAuthenticating,
    loginWithMachineKey,
    getTimeRemaining,
    isTokenExpired,
  } = useZeroIdAuth();
  
  // Wizard state
  const [currentStep, setCurrentStep] = useState(0);
  
  // Combined machine key + enrollment state (single atomic operation)
  const [deviceSetupState, setDeviceSetupState] = useState<StepState>('idle');
  const [deviceSetupError, setDeviceSetupError] = useState<string | null>(null);
  
  // Retry state for individual operations
  const [enrollmentRetrying, setEnrollmentRetrying] = useState(false);
  const [sessionRetrying, setSessionRetrying] = useState(false);
  const [enrollmentError, setEnrollmentError] = useState<string | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  
  // Password entry state (kept until machine key is created)
  const [isGenerating, setIsGenerating] = useState(false);
  const [password, setPassword] = useState('');
  const [passwordConfirm, setPasswordConfirm] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [showPasswordConfirm, setShowPasswordConfirm] = useState(false);

  // Password validation
  const passwordValidation = useMemo(() => {
    const isTooShort = password.length > 0 && password.length < MIN_PASSWORD_LENGTH;
    const passwordsMatch = password === passwordConfirm;
    const isValid = password.length >= MIN_PASSWORD_LENGTH && passwordsMatch;
    return { isTooShort, passwordsMatch, isValid };
  }, [password, passwordConfirm]);

  // Handle generate - generates the neural key (password kept for machine key creation)
  const handleGenerate = useCallback(async () => {
    if (!passwordValidation.isValid) return false;
    
    setIsGenerating(true);
    try {
      await generateNeuralKey(password);
      return true;
    } catch (err) {
      console.error('Failed to generate Neural Key:', err);
      return false;
    } finally {
      setIsGenerating(false);
    }
  }, [generateNeuralKey, password, passwordValidation.isValid]);

  // Handle step change - intercept to trigger generation when leaving password step
  const handleStepChange = useCallback(async (newStep: number) => {
    if (currentStep === 1 && newStep === 2) {
      const success = await handleGenerate();
      if (success) {
        setCurrentStep(newStep);
      }
      return;
    }
    setCurrentStep(newStep);
  }, [currentStep, handleGenerate]);

  // Handle combined device setup (creates machine key AND enrolls with ZID atomically)
  const handleDeviceSetup = useCallback(async () => {
    if (!state.pendingShards || state.pendingShards.length === 0) {
      setDeviceSetupError('No shards available');
      setDeviceSetupState('error');
      return;
    }
    if (!password) {
      setDeviceSetupError('Password is required');
      setDeviceSetupState('error');
      return;
    }

    setDeviceSetupState('loading');
    setDeviceSetupError(null);

    try {
      const shard = state.pendingShards[0];
      await createMachineKeyAndEnroll(
        'This Device',
        undefined,
        undefined,
        shard,
        password,
        DEFAULT_ZID_ENDPOINT
      );
      setDeviceSetupState('success');
      setPassword('');
      setPasswordConfirm('');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Device setup failed';
      setDeviceSetupError(errorMsg);
      setDeviceSetupState('error');
    }
  }, [state.pendingShards, password, createMachineKeyAndEnroll]);

  // Handle copy all shards to clipboard
  const handleCopyAll = useCallback(() => {
    if (!state.pendingShards) return;
    const formattedShards = state.pendingShards
      .map((shard) => `Shard ${shard.index}: ${shard.hex}`)
      .join('\n');
    const text = `Neural Key Recovery Shards (1 of 3 + password required)\n${'='.repeat(50)}\n${formattedShards}`;
    copy(text, 'all');
  }, [state.pendingShards, copy]);

  // Handle "I've saved my shards" confirmation
  const handleConfirmSaved = useCallback(() => {
    confirmShardsSaved();
  }, [confirmShardsSaved]);

  // ===========================================================================
  // Setup Status - Derived state and retry handlers for active view
  // ===========================================================================
  
  const hasMachineKey = machineKeysState.machines.length > 0;
  const isEnrolled = remoteAuthState !== null;
  
  const sessionStatus = useMemo(() => {
    if (!remoteAuthState) return 'none' as const;
    if (isTokenExpired()) return 'expired' as const;
    return 'active' as const;
  }, [remoteAuthState, isTokenExpired]);
  
  const handleRetryEnrollment = useCallback(async () => {
    setEnrollmentRetrying(true);
    setEnrollmentError(null);
    try {
      await loginWithMachineKey(DEFAULT_ZID_ENDPOINT);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Connection failed';
      setEnrollmentError(errorMsg);
    } finally {
      setEnrollmentRetrying(false);
    }
  }, [loginWithMachineKey]);
  
  const handleRetrySession = useCallback(async () => {
    setSessionRetrying(true);
    setSessionError(null);
    try {
      await loginWithMachineKey(DEFAULT_ZID_ENDPOINT);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Session refresh failed';
      setSessionError(errorMsg);
    } finally {
      setSessionRetrying(false);
    }
  }, [loginWithMachineKey]);

  // Handle wizard finish
  const handleWizardFinish = useCallback(() => {
    confirmShardsSaved();
  }, [confirmShardsSaved]);

  // Build wizard steps array
  const wizardSteps: WizardStep[] = useMemo(
    () => [
      {
        id: 'intro',
        label: 'Introduction',
        content: (
          <div className={styles.wizardStepContent}>
            <div className={styles.identityHero}>
              <div className={styles.heroIcon}>
                <Brain size={48} strokeWidth={1} />
              </div>
              <Text size="md" className={styles.heroTitle}>
                Your Neural Key is Your Identity
              </Text>
              <Text size="sm" className={styles.heroDescription}>
                A Neural Key is a cryptographic identity that represents you across all devices.
                You'll receive 3 backup shards to store securely. To access your identity on a new
                device, you'll need 1 shard plus your password.
              </Text>
            </div>
          </div>
        ),
        isComplete: true,
        nextLabel: 'Generate Neural Key',
      },
      {
        id: 'password',
        label: 'Password',
        content: (
          <div className={styles.wizardStepContent}>
            <div className={styles.stepHeader}>
              <Text size="md" className={styles.stepTitle}>Create Your Password</Text>
              <Text size="sm" className={styles.stepDescription}>
                This password encrypts your Neural Key. You'll need it along with a recovery shard to
                restore your identity on a new device.
              </Text>
            </div>

            <div className={styles.passwordSection}>
              <div className={styles.passwordField}>
                <Label size="xs">Password (min {MIN_PASSWORD_LENGTH} characters)</Label>
                <div className={styles.passwordInputWrapper}>
                  <Input
                    type={showPassword ? 'text' : 'password'}
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="Enter a secure password"
                    className={styles.passwordInput}
                  />
                  <button
                    type="button"
                    className={styles.passwordToggle}
                    onClick={() => setShowPassword(!showPassword)}
                  >
                    {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
                {passwordValidation.isTooShort && (
                  <Text size="xs" className={styles.passwordError}>
                    Password must be at least {MIN_PASSWORD_LENGTH} characters
                  </Text>
                )}
              </div>

              <div className={styles.passwordField}>
                <Label size="xs">Confirm Password</Label>
                <div className={styles.passwordInputWrapper}>
                  <Input
                    type={showPasswordConfirm ? 'text' : 'password'}
                    value={passwordConfirm}
                    onChange={(e) => setPasswordConfirm(e.target.value)}
                    placeholder="Confirm your password"
                    className={styles.passwordInput}
                  />
                  <button
                    type="button"
                    className={styles.passwordToggle}
                    onClick={() => setShowPasswordConfirm(!showPasswordConfirm)}
                  >
                    {showPasswordConfirm ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
                {passwordConfirm.length > 0 && !passwordValidation.passwordsMatch && (
                  <Text size="xs" className={styles.passwordError}>
                    Passwords do not match
                  </Text>
                )}
              </div>
            </div>
          </div>
        ),
        isComplete: passwordValidation.isValid,
        nextLabel: isGenerating ? 'Generating...' : 'Generate',
        nextDisabled: isGenerating,
      },
      {
        id: 'device-setup',
        label: 'Device Setup',
        content: (
          <div className={styles.wizardStepContent}>
            <div className={styles.verificationContainer}>
              {deviceSetupState === 'loading' && (
                <div className={styles.verificationStatus}>
                  <div className={styles.verificationIcon}>
                    <Loader size={32} className={styles.spinner} />
                  </div>
                  <Text size="md" className={styles.verificationTitle}>Setting Up Device</Text>
                  <Text size="sm" className={styles.verificationDescription}>
                    Creating device key and registering with ZERO ID...
                  </Text>
                </div>
              )}

              {deviceSetupState === 'success' && (
                <div className={styles.verificationStatus}>
                  <div className={styles.verificationIconSuccess}>
                    <ShieldCheck size={32} />
                  </div>
                  <Text size="md" className={styles.verificationTitle}>Device Setup Complete</Text>
                  <Text size="sm" className={styles.verificationDescription}>
                    This device is now authorized and registered with ZERO ID.
                  </Text>
                </div>
              )}

              {deviceSetupState === 'error' && (
                <div className={styles.verificationStatus}>
                  <div className={styles.verificationIconError}>
                    <AlertTriangle size={32} />
                  </div>
                  <Text size="md" className={styles.verificationTitle}>Device Setup Failed</Text>
                  <Text size="sm" className={styles.verificationDescription}>
                    {deviceSetupError || 'Failed to set up device.'}
                  </Text>
                  <div className={styles.retryContainer}>
                    <Button
                      variant="secondary"
                      size="md"
                      onClick={() => {
                        setDeviceSetupState('idle');
                        handleDeviceSetup();
                      }}
                    >
                      <RefreshCw size={16} />
                      Retry
                    </Button>
                  </div>
                </div>
              )}

              {deviceSetupState === 'idle' && (
                <div className={styles.verificationStatus}>
                  <div className={styles.verificationIcon}>
                    <Laptop size={32} />
                  </div>
                  <Text size="md" className={styles.verificationTitle}>Set Up This Device</Text>
                  <Text size="sm" className={styles.verificationDescription}>
                    Create a device key and register with ZERO ID for cross-device sync.
                  </Text>
                  <div className={styles.verifyButtonContainer}>
                    <Button
                      variant="primary"
                      size="md"
                      onClick={handleDeviceSetup}
                    >
                      <Laptop size={16} />
                      Set Up Device
                    </Button>
                  </div>
                </div>
              )}
            </div>
          </div>
        ),
        isComplete: deviceSetupState === 'success',
        nextLabel: deviceSetupState === 'loading' ? 'Setting up...' : deviceSetupState === 'success' ? 'Next' : 'Skip',
        nextDisabled: deviceSetupState === 'loading',
      },
      {
        id: 'backup',
        label: 'Backup',
        content: state.pendingShards ? (
          <ShardDisplay
            shards={state.pendingShards}
            isCopied={isCopied}
            onCopyAll={handleCopyAll}
            onCopyShard={(hex, key) => copy(hex, key)}
          />
        ) : null,
        isComplete: true,
      },
    ],
    [
      showPassword,
      password,
      passwordValidation,
      showPasswordConfirm,
      passwordConfirm,
      isGenerating,
      deviceSetupState,
      deviceSetupError,
      handleDeviceSetup,
      state.pendingShards,
      isCopied,
      handleCopyAll,
      copy,
    ]
  );

  // =========================================================================
  // Render Logic
  // =========================================================================

  // Show nothing during initial settling period
  if (state.isInitializing) {
    return null;
  }

  // Show loading state
  if (state.isLoading && !isGenerating) {
    return (
      <div className={styles.panelContainer}>
        <GroupCollapsible title="Neural Key" defaultOpen className={styles.collapsibleSection}>
          <div className={styles.identitySection}>
            <div className={styles.loadingState}>
              <Loader size={24} className={styles.spinner} />
              <Text size="sm">Loading Neural Key status...</Text>
            </div>
          </div>
        </GroupCollapsible>
      </div>
    );
  }

  // Show error state
  if (state.error) {
    return (
      <div className={styles.panelContainer}>
        <GroupCollapsible title="Neural Key" defaultOpen className={styles.collapsibleSection}>
          <div className={styles.identitySection}>
            <Card className={styles.dangerCard}>
              <CardItem
                icon={<AlertTriangle size={16} />}
                title="Error"
                description={state.error}
                className={styles.dangerCardItem}
              />
            </Card>
          </div>
        </GroupCollapsible>
      </div>
    );
  }

  // Show wizard when no key exists or when pending shards need backup
  if (!state.hasNeuralKey || state.pendingShards) {
    return (
      <div className={styles.panelContainer}>
        <PanelWizard
          steps={wizardSteps}
          currentStep={currentStep}
          onStepChange={handleStepChange}
          onFinish={handleWizardFinish}
          finishLabel="I've Saved My Shards"
          showSteps={true}
          showFooter={true}
          background="none"
          border="none"
          className={styles.wizardPanel}
        />
      </div>
    );
  }

  // Active neural key - show status
  return (
    <NeuralKeyStatus
      publicIdentifiers={state.publicIdentifiers}
      createdAt={state.createdAt}
      hasMachineKey={hasMachineKey}
      isEnrolled={isEnrolled}
      sessionStatus={sessionStatus}
      enrollmentRetrying={enrollmentRetrying}
      sessionRetrying={sessionRetrying}
      enrollmentError={enrollmentError}
      sessionError={sessionError}
      isAuthenticating={isAuthenticating}
      getTimeRemaining={getTimeRemaining}
      onRetryEnrollment={handleRetryEnrollment}
      onRetrySession={handleRetrySession}
    />
  );
}
