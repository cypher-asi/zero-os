import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { createElement } from 'react';
import { useIdentityServiceClient } from '../useIdentityServiceClient';
import { SupervisorProvider } from '../useSupervisor';
import { createMockSupervisor } from '../../../test/mocks';

// Mock the stores
const mockCurrentUser = { id: '12345', displayName: 'Test User' };
let mockSelectCurrentUser = vi.fn(() => mockCurrentUser);

vi.mock('../../../stores', () => ({
  useIdentityStore: (selector: (s: Record<string, unknown>) => unknown) =>
    selector({ currentUser: mockSelectCurrentUser() }),
  selectCurrentUser: (state: Record<string, unknown>) => state.currentUser,
}));

// Mock the IdentityServiceClient
const mockIdentityServiceClient = {
  createMachineKey: vi.fn(),
  listMachineKeys: vi.fn(),
  revokeMachineKey: vi.fn(),
  rotateMachineKey: vi.fn(),
};

vi.mock('../../../services', () => ({
  IdentityServiceClient: vi.fn().mockImplementation(() => mockIdentityServiceClient),
  userIdToBigInt: vi.fn((id: string | number | bigint | null | undefined) => {
    if (id == null) return null;
    if (typeof id === 'bigint') return id;
    return BigInt(id);
  }),
}));

function createWrapper(supervisor: ReturnType<typeof createMockSupervisor> | null) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(SupervisorProvider, { value: supervisor }, children);
  };
}

describe('useIdentityServiceClient', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSelectCurrentUser = vi.fn(() => mockCurrentUser);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initialization', () => {
    it('returns null client when supervisor is not available', () => {
      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(null),
      });

      expect(result.current.client).toBeNull();
      expect(result.current.isReady).toBe(false);
    });

    it('throws when getting client with no supervisor', () => {
      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(null),
      });

      expect(() => result.current.getClientOrThrow()).toThrow(
        'Identity service client not available'
      );
    });
  });

  describe('userId', () => {
    it('returns userId as BigInt when user is logged in', () => {
      const mockSupervisor = createMockSupervisor();

      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      expect(result.current.userId).toBe(BigInt(12345));
    });

    it('returns null userId when no user is logged in', () => {
      mockSelectCurrentUser = vi.fn(() => null);
      const mockSupervisor = createMockSupervisor();

      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      expect(result.current.userId).toBeNull();
    });
  });

  describe('isReady', () => {
    it('is false when supervisor is not available', () => {
      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(null),
      });

      expect(result.current.isReady).toBe(false);
    });

    it('is false when user is not logged in', () => {
      mockSelectCurrentUser = vi.fn(() => null);
      const mockSupervisor = createMockSupervisor();

      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      // isReady requires both client AND userId to be present
      // Since userId is null, isReady should be false regardless of client
      expect(result.current.isReady).toBe(false);
    });
  });

  describe('getClientOrThrow', () => {
    it('throws when client is not available', () => {
      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(null),
      });

      expect(() => result.current.getClientOrThrow()).toThrow(
        'Identity service client not available'
      );
    });

    it('returns client when available (after effect runs)', () => {
      const mockSupervisor = createMockSupervisor();

      const { result, rerender } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      // Trigger a re-render to pick up the effect result
      rerender();

      // After rerender, the client should be available
      const client = result.current.getClientOrThrow();
      expect(client).toBeDefined();
    });
  });

  describe('getUserIdOrThrow', () => {
    it('throws when no user is logged in', () => {
      mockSelectCurrentUser = vi.fn(() => null);
      const mockSupervisor = createMockSupervisor();

      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      expect(() => result.current.getUserIdOrThrow()).toThrow('No user logged in');
    });

    it('returns userId when user is logged in', () => {
      const mockSupervisor = createMockSupervisor();

      const { result } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      const userId = result.current.getUserIdOrThrow();
      expect(userId).toBe(BigInt(12345));
    });
  });

  describe('stability', () => {
    it('getClientOrThrow callback reference is stable', () => {
      const mockSupervisor = createMockSupervisor();

      const { result, rerender } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      const firstCallback = result.current.getClientOrThrow;

      rerender();

      expect(result.current.getClientOrThrow).toBe(firstCallback);
    });

    it('getUserIdOrThrow callback reference changes when userId changes', () => {
      const mockSupervisor = createMockSupervisor();

      const { result, rerender } = renderHook(() => useIdentityServiceClient(), {
        wrapper: createWrapper(mockSupervisor),
      });

      const firstCallback = result.current.getUserIdOrThrow;

      // Change user ID
      mockSelectCurrentUser = vi.fn(() => ({ id: '99999', displayName: 'New User' }));

      rerender();

      // Callback should be different because userId dependency changed
      expect(result.current.getUserIdOrThrow).not.toBe(firstCallback);
    });
  });
});
