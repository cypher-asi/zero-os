/**
 * AuthGate Component
 *
 * Checks for a valid ZID session and shows the AuthPanel modal
 * when no valid session exists. The modal cannot be dismissed
 * in gate mode - user must authenticate.
 *
 * Sessions persist across refreshes via VFS, so authenticated
 * users see the desktop directly without needing to re-login.
 */

import { useIdentityStore, selectRemoteAuthState } from '@/stores';
import { useZeroIdAuth } from '../hooks/useZeroIdAuth';
import { AuthPanel } from '../Taskbar/IdentityPanel/modals/AuthPanel';

interface AuthGateProps {
  children: React.ReactNode;
}

export function AuthGate({ children }: AuthGateProps) {
  const remoteAuthState = useIdentityStore(selectRemoteAuthState);
  const { isLoadingSession } = useZeroIdAuth();

  // Wait for session check to complete before showing login
  // During loading, render children so the desktop can initialize
  if (isLoadingSession) {
    return <>{children}</>;
  }

  // No session - show auth modal (cannot be dismissed)
  if (!remoteAuthState) {
    return (
      <>
        {children}
        <AuthPanel
          initialView="login"
          onClose={() => {
            // No-op: Gate mode doesn't allow dismissing
          }}
          dismissable={false}
        />
      </>
    );
  }

  // Valid session exists - render children normally
  return <>{children}</>;
}
