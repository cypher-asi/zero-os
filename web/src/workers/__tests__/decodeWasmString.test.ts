/**
 * Tests for decodeWasmString
 *
 * Tests for the WASM string decoding utility that handles SharedArrayBuffer.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { decodeWasmString } from '../wasm-bindgen-shims';
import { writeString } from './helpers';

describe('decodeWasmString', () => {
  describe('with regular ArrayBuffer', () => {
    let memory: WebAssembly.Memory;

    beforeEach(() => {
      memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    });

    it('should decode a simple ASCII string', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'Hello World');

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('Hello World');
    });

    it('should decode UTF-8 special characters', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, '日本語 🎉 émojis');

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('日本語 🎉 émojis');
    });

    it('should decode an empty string', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, '');

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('');
    });

    it('should handle zero length', () => {
      const ptr = 1024;
      const result = decodeWasmString(memory, ptr, 0);
      expect(result).toBe('');
    });
  });

  describe('with SharedArrayBuffer', () => {
    let memory: WebAssembly.Memory;

    beforeEach(() => {
      memory = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true });
    });

    it('should decode a string from shared memory by copying', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'Shared Memory String');

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('Shared Memory String');
    });

    it('should decode UTF-8 from shared memory', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'こんにちは 世界');

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('こんにちは 世界');
    });
  });

  describe('with null memory', () => {
    it('should return empty string when memory is null', () => {
      const result = decodeWasmString(null, 1024, 10);
      expect(result).toBe('');
    });
  });

  describe('edge cases', () => {
    it('should handle strings with null bytes', () => {
      const memory = new WebAssembly.Memory({ initial: 1 });
      const ptr = 1024;
      const str = 'before\0after';
      const len = writeString(memory, ptr, str);

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe('before\0after');
    });

    it('should handle very long strings', () => {
      const memory = new WebAssembly.Memory({ initial: 2 });
      const ptr = 1024;
      const longStr = 'x'.repeat(10000);
      const len = writeString(memory, ptr, longStr);

      const result = decodeWasmString(memory, ptr, len);
      expect(result).toBe(longStr);
    });
  });
});
