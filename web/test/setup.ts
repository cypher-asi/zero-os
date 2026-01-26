import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import '@testing-library/react';

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Mock Web Worker global scope for worker tests
if (typeof self === 'undefined') {
  (globalThis as unknown as { self: typeof globalThis }).self = globalThis;
}

// Mock crypto.getRandomValues for tests if not available
if (!globalThis.crypto) {
  globalThis.crypto = {
    getRandomValues: <T extends ArrayBufferView | null>(array: T): T => {
      if (array && 'length' in array) {
        const view = array as unknown as Uint8Array;
        for (let i = 0; i < view.length; i++) {
          view[i] = Math.floor(Math.random() * 256);
        }
      }
      return array;
    },
    subtle: {} as SubtleCrypto,
    randomUUID: () => 'test-uuid-1234-5678-9012-abcdef123456',
  } as Crypto;
}

// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock ResizeObserver
class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}

window.ResizeObserver = ResizeObserverMock;

// Mock requestAnimationFrame
window.requestAnimationFrame = vi.fn((cb) => {
  return setTimeout(cb, 16) as unknown as number;
});

window.cancelAnimationFrame = vi.fn((id) => {
  clearTimeout(id);
});

// Mock PointerEvent since jsdom doesn't support it fully
class PointerEventMock extends MouseEvent {
  pointerId: number;
  width: number;
  height: number;
  pressure: number;
  tangentialPressure: number;
  tiltX: number;
  tiltY: number;
  twist: number;
  pointerType: string;
  isPrimary: boolean;

  constructor(type: string, params: PointerEventInit = {}) {
    super(type, params);
    this.pointerId = params.pointerId ?? 0;
    this.width = params.width ?? 1;
    this.height = params.height ?? 1;
    this.pressure = params.pressure ?? 0;
    this.tangentialPressure = params.tangentialPressure ?? 0;
    this.tiltX = params.tiltX ?? 0;
    this.tiltY = params.tiltY ?? 0;
    this.twist = params.twist ?? 0;
    this.pointerType = params.pointerType ?? 'mouse';
    this.isPrimary = params.isPrimary ?? false;
  }
}

window.PointerEvent = PointerEventMock as unknown as typeof PointerEvent;
