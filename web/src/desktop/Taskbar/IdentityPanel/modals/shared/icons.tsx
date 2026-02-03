import type { ReactNode } from 'react';
import { Github } from 'lucide-react';

/**
 * Ethereum logo SVG icon
 */
export function EthereumIcon(): ReactNode {
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
 * Google logo SVG icon
 */
export function GoogleIcon(): ReactNode {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
      <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
      <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
      <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
      <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
    </svg>
  );
}

/**
 * X (Twitter) logo SVG icon
 */
export function XIcon(): ReactNode {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}

/**
 * Solana logo SVG icon
 */
export function SolanaIcon(): ReactNode {
  return (
    <svg width="20" height="20" viewBox="0 0 397.7 311.7" fill="currentColor">
      <path d="M64.6 237.9c2.4-2.4 5.7-3.8 9.2-3.8h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1l62.7-62.7z" />
      <path d="M64.6 3.8C67.1 1.4 70.4 0 73.8 0h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1L64.6 3.8z" />
      <path d="M333.1 120.1c-2.4-2.4-5.7-3.8-9.2-3.8H6.5c-5.8 0-8.7 7-4.6 11.1l62.7 62.7c2.4 2.4 5.7 3.8 9.2 3.8h317.4c5.8 0 8.7-7 4.6-11.1l-62.7-62.7z" />
    </svg>
  );
}

/**
 * Get provider icon based on provider name
 */
export function getProviderIcon(providerName: string): ReactNode {
  const name = providerName.toLowerCase();

  switch (name) {
    case 'google':
      return <GoogleIcon />;

    case 'github':
      return <Github size={20} />;

    case 'twitter':
    case 'x':
      return <XIcon />;

    case 'ethereum':
      return <EthereumIcon />;

    case 'solana':
      return <SolanaIcon />;

    default:
      return null;
  }
}

/**
 * Capitalize first letter of a string
 */
export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
