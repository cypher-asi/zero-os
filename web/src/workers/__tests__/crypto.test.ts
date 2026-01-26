/**
 * Tests for Crypto Utilities
 *
 * Tests for SharedArrayBuffer-safe crypto operations.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { getRandomValues, fillRandomArray } from '../crypto';
import { createTestState, createTestStateNonShared } from './helpers';

describe('getRandomValues', () => {
  const workerId = 1;
  const pid = 42;

  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('with regular ArrayBuffer', () => {
    it('should fill buffer with random values directly', () => {
      const state = createTestStateNonShared();
      const ptr = 1024;
      const len = 16;

      getRandomValues(state.wasmMemory!, ptr, len, workerId, pid);

      const result = new Uint8Array(state.wasmMemory!.buffer, ptr, len);
      // Should have non-zero values (statistically, all zeros is extremely unlikely)
      const hasNonZero = result.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });

    it('should fill the correct number of bytes', () => {
      const state = createTestStateNonShared();
      const ptr = 1024;
      const len = 32;

      // Clear the area first
      new Uint8Array(state.wasmMemory!.buffer, ptr, len + 16).fill(0);

      getRandomValues(state.wasmMemory!, ptr, len, workerId, pid);

      // Check bytes after the requested range weren't touched
      const after = new Uint8Array(state.wasmMemory!.buffer, ptr + len, 16);
      expect(after.every((b) => b === 0)).toBe(true);
    });
  });

  describe('with SharedArrayBuffer', () => {
    it('should fill buffer via copy for SharedArrayBuffer', () => {
      const state = createTestState();
      expect(state.wasmMemory!.buffer).toBeInstanceOf(SharedArrayBuffer);

      const ptr = 1024;
      const len = 16;

      getRandomValues(state.wasmMemory!, ptr, len, workerId, pid);

      const result = new Uint8Array(state.wasmMemory!.buffer, ptr, len);
      const hasNonZero = result.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });

    it('should work with various buffer sizes', () => {
      const state = createTestState();
      const sizes = [1, 8, 32, 64, 256];

      for (const size of sizes) {
        const ptr = 1024;
        getRandomValues(state.wasmMemory!, ptr, size, workerId, pid);

        const result = new Uint8Array(state.wasmMemory!.buffer, ptr, size);
        // Just verify no error thrown and length is correct
        expect(result.length).toBe(size);
      }
    });
  });

  describe('error handling', () => {
    it('should throw when memory is not provided', () => {
      expect(() => {
        getRandomValues(null as unknown as WebAssembly.Memory, 0, 16, workerId, pid);
      }).toThrow('Memory not initialized for getRandomValues');
    });
  });
});

describe('fillRandomArray', () => {
  const workerId = 1;
  const pid = 42;

  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('with regular ArrayBuffer', () => {
    it('should fill Uint8Array with random values', () => {
      const array = new Uint8Array(16);
      fillRandomArray(array, workerId, pid);

      const hasNonZero = array.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });
  });

  describe('with SharedArrayBuffer', () => {
    it('should fill array backed by SharedArrayBuffer via copy', () => {
      const sab = new SharedArrayBuffer(32);
      const array = new Uint8Array(sab);

      fillRandomArray(array, workerId, pid);

      const hasNonZero = array.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });
  });

  describe('different array sizes', () => {
    it('should handle single byte arrays', () => {
      const array = new Uint8Array(1);
      fillRandomArray(array, workerId, pid);
      // Just verify no error
      expect(array.length).toBe(1);
    });

    it('should handle large arrays', () => {
      const array = new Uint8Array(1024);
      fillRandomArray(array, workerId, pid);

      const hasNonZero = array.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });
  });
});
