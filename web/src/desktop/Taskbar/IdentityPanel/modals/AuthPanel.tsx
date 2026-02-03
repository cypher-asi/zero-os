import { useState, useEffect, useRef, useCallback } from 'react';
import { Panel } from '@cypher-asi/zui';
import { LoginContent } from './LoginModal';
import { RegisterContent } from './RegisterWizard';
import { AuthLink } from './AuthLink';
import { useAnimatedHeight } from '../../../hooks/useAnimatedHeight';
import styles from './AuthPanel.module.css';

export type AuthView = 'login' | 'register';

// Image URL shared across login and register views
const AUTH_HEADER_IMAGE = 'https://i.pinimg.com/1200x/f5/7f/40/f57f40ef5842c2332f3debebfe8b06b6.jpg';

interface AuthPanelProps {
  /** Initial view to show */
  initialView: AuthView;
  /** Callback when panel should close */
  onClose: () => void;
  /** Callback when self-sovereign identity is selected during registration */
  onSelfSovereignSelected?: () => void;
}

/**
 * AuthPanel - Unified panel for login and registration
 *
 * Layout:
 * - Shared image header (fixed height)
 * - Dynamic content area (login form or registration wizard)
 * - Shared AuthLink footer (fixed at bottom)
 */
export function AuthPanel({ initialView, onClose, onSelfSovereignSelected }: AuthPanelProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const [currentView, setCurrentView] = useState<AuthView>(initialView);
  const [isLoading, setIsLoading] = useState(false);
  const { containerStyle, contentRef } = useAnimatedHeight();

  // Switch to registration view
  const handleSwitchToRegister = useCallback(() => {
    setCurrentView('register');
  }, []);

  // Switch to login view
  const handleSwitchToLogin = useCallback(() => {
    setCurrentView('login');
  }, []);

  // Track loading state from child components
  const handleLoadingChange = useCallback((loading: boolean) => {
    setIsLoading(loading);
  }, []);

  // Click outside to close (only on overlay, not panel content)
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

  return (
    <div ref={overlayRef} className={styles.overlay}>
      <Panel
        variant="glass"
        border="future"
        className={styles.panel}
      >
        {/* Shared Image Header */}
        <div className={styles.imageHeader}>
          <img src={AUTH_HEADER_IMAGE} alt="" />
        </div>

        {/* Dynamic Content Area - Animated height container */}
        <div className={styles.content} style={containerStyle}>
          <div ref={contentRef} className={styles.contentWrapper}>
            {currentView === 'login' ? (
              <LoginContent
                onClose={onClose}
                onLoadingChange={handleLoadingChange}
              />
            ) : (
              <RegisterContent
                onClose={onClose}
                onSelfSovereignSelected={onSelfSovereignSelected}
                onLoadingChange={handleLoadingChange}
              />
            )}
          </div>
        </div>

        {/* Shared Footer with AuthLink */}
        <div className={styles.footer}>
          {currentView === 'login' ? (
            <AuthLink
              text="Don't have an account?"
              linkText="Create Account"
              onClick={handleSwitchToRegister}
              disabled={isLoading}
            />
          ) : (
            <AuthLink
              text="Already have an account?"
              linkText="Sign In"
              onClick={handleSwitchToLogin}
              disabled={isLoading}
            />
          )}
        </div>
      </Panel>
    </div>
  );
}
