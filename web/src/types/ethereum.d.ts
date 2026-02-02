/**
 * MetaMask / Ethereum Provider Type Declarations
 *
 * Provides TypeScript types for the `window.ethereum` object injected by
 * MetaMask and other Ethereum wallet browser extensions.
 *
 * @see https://docs.metamask.io/guide/ethereum-provider.html
 */

/**
 * Ethereum JSON-RPC request parameters for common methods
 */
interface EthRequestAccountsParams {
  method: 'eth_requestAccounts';
  params?: never;
}

interface PersonalSignParams {
  method: 'personal_sign';
  params: [message: string, address: string];
}

interface EthAccountsParams {
  method: 'eth_accounts';
  params?: never;
}

interface EthChainIdParams {
  method: 'eth_chainId';
  params?: never;
}

interface WalletSwitchChainParams {
  method: 'wallet_switchEthereumChain';
  params: [{ chainId: string }];
}

interface GenericEthRequestParams {
  method: string;
  params?: unknown[];
}

type EthereumRequestArgs =
  | EthRequestAccountsParams
  | PersonalSignParams
  | EthAccountsParams
  | EthChainIdParams
  | WalletSwitchChainParams
  | GenericEthRequestParams;

/**
 * Ethereum provider events
 */
interface EthereumProviderEvents {
  accountsChanged: (accounts: string[]) => void;
  chainChanged: (chainId: string) => void;
  connect: (info: { chainId: string }) => void;
  disconnect: (error: { code: number; message: string }) => void;
  message: (message: { type: string; data: unknown }) => void;
}

/**
 * Ethereum Provider interface (EIP-1193)
 *
 * This is the interface for the `window.ethereum` object.
 */
interface EthereumProvider {
  /**
   * Whether the provider is MetaMask
   */
  isMetaMask?: boolean;

  /**
   * Whether the provider is connected to the current chain
   */
  isConnected(): boolean;

  /**
   * Send a JSON-RPC request to the Ethereum provider
   *
   * @param args - The request arguments containing method and params
   * @returns Promise resolving to the request result
   *
   * @example
   * // Request account access
   * const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
   *
   * @example
   * // Sign a message with personal_sign (EIP-191)
   * const signature = await window.ethereum.request({
   *   method: 'personal_sign',
   *   params: [message, address],
   * });
   */
  request<T = unknown>(args: EthereumRequestArgs): Promise<T>;

  /**
   * Add an event listener
   */
  on<K extends keyof EthereumProviderEvents>(
    event: K,
    listener: EthereumProviderEvents[K]
  ): void;

  /**
   * Remove an event listener
   */
  removeListener<K extends keyof EthereumProviderEvents>(
    event: K,
    listener: EthereumProviderEvents[K]
  ): void;

  /**
   * Selected address (may be null if not connected)
   */
  selectedAddress: string | null;

  /**
   * Chain ID as hex string (e.g., "0x1" for mainnet)
   */
  chainId: string | null;
}

/**
 * Extend the global Window interface to include ethereum
 */
declare global {
  interface Window {
    /**
     * Ethereum provider injected by MetaMask or other wallet extensions
     */
    ethereum?: EthereumProvider;
  }
}

export {};
