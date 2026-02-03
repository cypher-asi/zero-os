/**
 * AuthGate Component
 *
 * Checks for a valid ZID session and shows the AuthPanel modal
 * when no valid session exists. The modal cannot be dismissed
 * in gate mode - user must authenticate.
 *
 * Sessions persist across refreshes via VFS, so authenticated
 * users see the desktop directly without needing to re-login.
 *
 * Security: Passes `isLocked` prop to children when no session exists.
 * This disables all desktop interactions at the JavaScript level,
 * preventing bypass via DevTools DOM manipulation.
 */

import React from 'react';
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
  // During loading, render locked desktop so it can initialize
  if (isLoadingSession) {
    return <>{React.cloneElement(children as React.ReactElement, { isLocked: true })}</>;
  }

  // No session - show auth modal (cannot be dismissed)
  // Pass isLocked=true to disable all desktop interactions
  if (!remoteAuthState) {
    return (
      <>
        {React.cloneElement(children as React.ReactElement, { isLocked: true })}
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

  // Valid session exists - render children normally (unlocked)
  return <>{React.cloneElement(children as React.ReactElement, { isLocked: false })}</>;
}
