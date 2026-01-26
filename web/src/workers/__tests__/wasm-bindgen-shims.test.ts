/**
 * Tests for wasm-bindgen Compatibility Shims
 *
 * Comprehensive tests for all wasm-bindgen shim functions including
 * base shims, proxy handler patterns, and externref shims.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { WasmBindgenHeap } from '../heap';
import {
  createWasmBindgenShims,
  createWasmBindgenProxy,
  createExternrefShims,
  createExternrefProxy,
} from '../wasm-bindgen-shims';
import { createTestState, writeString } from './helpers';

describe('createWasmBindgenShims', () => {
  let heap: WasmBindgenHeap;
  let memory: WebAssembly.Memory;
  let shims: ReturnType<typeof createWasmBindgenShims>;
  const workerId = 1;
  const pid = 42;

  beforeEach(() => {
    heap = new WasmBindgenHeap();
    memory = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true });
    shims = createWasmBindgenShims(heap, () => memory, workerId, pid);
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('__wbindgen_throw', () => {
    it('should throw an Error with decoded message', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'Test error message');

      expect(() => {
        (shims.__wbindgen_throw as (ptr: number, len: number) => never)(ptr, len);
      }).toThrow('Test error message');
    });

    it('should throw error when memory is not initialized', () => {
      const nullMemoryShims = createWasmBindgenShims(
        heap,
        () => null,
        workerId,
        pid
      );

      expect(() => {
        (nullMemoryShims.__wbindgen_throw as (ptr: number, len: number) => never)(0, 0);
      }).toThrow('wasm-bindgen error: memory not initialized');
    });
  });

  describe('__wbindgen_rethrow', () => {
    it('should rethrow the provided error', () => {
      const error = new Error('original error');

      expect(() => {
        (shims.__wbindgen_rethrow as (arg: unknown) => never)(error);
      }).toThrow('original error');
    });
  });

  describe('__wbindgen_memory', () => {
    it('should return the WASM memory', () => {
      const result = (shims.__wbindgen_memory as () => WebAssembly.Memory | null)();
      expect(result).toBe(memory);
    });
  });

  describe('__wbindgen_is_undefined', () => {
    it('should return true for undefined', () => {
      // Index 128 is pre-populated with undefined
      const result = (shims.__wbindgen_is_undefined as (idx: number) => boolean)(128);
      expect(result).toBe(true);
    });

    it('should return false for non-undefined values', () => {
      const idx = heap.addObject('hello');
      const result = (shims.__wbindgen_is_undefined as (idx: number) => boolean)(idx);
      expect(result).toBe(false);
    });
  });

  describe('__wbindgen_is_null', () => {
    it('should return true for null', () => {
      // Index 129 is pre-populated with null
      const result = (shims.__wbindgen_is_null as (idx: number) => boolean)(129);
      expect(result).toBe(true);
    });

    it('should return false for non-null values', () => {
      const idx = heap.addObject({ test: true });
      const result = (shims.__wbindgen_is_null as (idx: number) => boolean)(idx);
      expect(result).toBe(false);
    });
  });

  describe('__wbindgen_object_drop_ref', () => {
    it('should drop an object from the heap', () => {
      const obj = { test: 'value' };
      const idx = heap.addObject(obj);

      (shims.__wbindgen_object_drop_ref as (idx: number) => void)(idx);

      // Slot should be reused for next add
      const newObj = { new: 'object' };
      const newIdx = heap.addObject(newObj);
      expect(newIdx).toBe(idx);
    });
  });

  describe('__wbindgen_object_clone_ref', () => {
    it('should clone an object reference', () => {
      const obj = { test: 'value' };
      const idx = heap.addObject(obj);

      const clonedIdx = (shims.__wbindgen_object_clone_ref as (idx: number) => number)(idx);

      expect(clonedIdx).not.toBe(idx);
      expect(heap.getObject(clonedIdx)).toBe(obj);
    });
  });

  describe('__wbindgen_string_new', () => {
    it('should decode string and add to heap', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'Hello, WASM!');

      const idx = (shims.__wbindgen_string_new as (ptr: number, len: number) => number)(ptr, len);

      expect(heap.getObject(idx)).toBe('Hello, WASM!');
    });
  });

  describe('__wbindgen_error_new', () => {
    it('should create Error object on heap', () => {
      const ptr = 1024;
      const len = writeString(memory, ptr, 'Custom error');

      const idx = (shims.__wbindgen_error_new as (ptr: number, len: number) => number)(ptr, len);

      const error = heap.getObject(idx) as Error;
      expect(error).toBeInstanceOf(Error);
      expect(error.message).toBe('Custom error');
    });

    it('should use default message for empty string', () => {
      const ptr = 1024;
      const idx = (shims.__wbindgen_error_new as (ptr: number, len: number) => number)(ptr, 0);

      const error = heap.getObject(idx) as Error;
      expect(error.message).toBe('Unknown error');
    });
  });

  describe('__wbindgen_jsval_eq', () => {
    it('should return true for equal values', () => {
      const result = (shims.__wbindgen_jsval_eq as (a: unknown, b: unknown) => boolean)(42, 42);
      expect(result).toBe(true);
    });

    it('should return false for different values', () => {
      const result = (shims.__wbindgen_jsval_eq as (a: unknown, b: unknown) => boolean)(1, 2);
      expect(result).toBe(false);
    });

    it('should use strict equality', () => {
      const result = (shims.__wbindgen_jsval_eq as (a: unknown, b: unknown) => boolean)('42', 42);
      expect(result).toBe(false);
    });
  });

  describe('__wbindgen_number_get', () => {
    it('should return number for number values', () => {
      const result = (shims.__wbindgen_number_get as (val: unknown) => number | undefined)(42);
      expect(result).toBe(42);
    });

    it('should return undefined for non-number values', () => {
      const result = (shims.__wbindgen_number_get as (val: unknown) => number | undefined)('hello');
      expect(result).toBeUndefined();
    });
  });

  describe('__wbindgen_boolean_get', () => {
    it('should return boolean for boolean values', () => {
      expect((shims.__wbindgen_boolean_get as (val: unknown) => boolean | undefined)(true)).toBe(true);
      expect((shims.__wbindgen_boolean_get as (val: unknown) => boolean | undefined)(false)).toBe(false);
    });

    it('should return undefined for non-boolean values', () => {
      const result = (shims.__wbindgen_boolean_get as (val: unknown) => boolean | undefined)(1);
      expect(result).toBeUndefined();
    });
  });

  describe('__wbindgen_cb_drop', () => {
    it('should return true', () => {
      const result = (shims.__wbindgen_cb_drop as (idx: number) => boolean)(0);
      expect(result).toBe(true);
    });
  });

  describe('__wbindgen_get_random_values', () => {
    it('should fill memory with random values', () => {
      const ptr = 1024;
      const len = 16;

      (shims.__wbindgen_get_random_values as (ptr: number, len: number) => void)(ptr, len);

      const result = new Uint8Array(memory.buffer, ptr, len);
      const hasNonZero = result.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });

    it('should throw when memory is null', () => {
      const nullMemoryShims = createWasmBindgenShims(
        heap,
        () => null,
        workerId,
        pid
      );

      expect(() => {
        (nullMemoryShims.__wbindgen_get_random_values as (ptr: number, len: number) => void)(0, 16);
      }).toThrow('Memory not initialized for getRandomValues');
    });
  });
});

describe('createWasmBindgenProxy', () => {
  let heap: WasmBindgenHeap;
  let memory: WebAssembly.Memory;
  let baseShims: ReturnType<typeof createWasmBindgenShims>;
  let proxy: Record<string, (...args: unknown[]) => unknown>;
  const workerId = 1;
  const pid = 42;

  beforeEach(() => {
    heap = new WasmBindgenHeap();
    memory = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true });
    baseShims = createWasmBindgenShims(heap, () => memory, workerId, pid);
    const handler = createWasmBindgenProxy(baseShims, heap, () => memory, workerId, pid);
    proxy = new Proxy(baseShims, handler);
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('existing shims passthrough', () => {
    it('should return existing shim functions', () => {
      expect(typeof proxy.__wbindgen_throw).toBe('function');
      expect(typeof proxy.__wbindgen_is_undefined).toBe('function');
    });
  });

  describe('crypto pattern matching', () => {
    it('should match crypto getter pattern', () => {
      const fn = proxy['__wbg_crypto_abc123'] as (selfIdx: number) => number;
      expect(typeof fn).toBe('function');

      // Add self to heap
      const selfIdx = heap.addObject({ crypto: globalThis.crypto });
      const cryptoIdx = fn(selfIdx);

      expect(cryptoIdx).toBeGreaterThanOrEqual(132);
    });

    it('should match getRandomValues pattern', () => {
      const fn = proxy['__wbg_getRandomValues_abc123'] as (
        cryptoIdx: number,
        arrayIdx: number
      ) => void;
      expect(typeof fn).toBe('function');

      // Set up heap objects
      const array = new Uint8Array(16);
      const cryptoIdx = heap.addObject(globalThis.crypto);
      const arrayIdx = heap.addObject(array);

      fn(cryptoIdx, arrayIdx);

      const hasNonZero = array.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });

    it('should match randomFillSync pattern', () => {
      const fn = proxy['__wbg_randomFillSync_abc123'] as (
        cryptoObj: unknown,
        ptr: number,
        len: number
      ) => void;
      expect(typeof fn).toBe('function');

      const ptr = 1024;
      const len = 16;
      fn(null, ptr, len);

      const result = new Uint8Array(memory.buffer, ptr, len);
      const hasNonZero = result.some((b) => b !== 0);
      expect(hasNonZero).toBe(true);
    });
  });

  describe('global accessor handlers', () => {
    it('should match SELF accessor pattern', () => {
      const fn = proxy['__wbg_static_accessor_SELF_abc123'] as () => number;
      expect(typeof fn).toBe('function');

      const idx = fn();
      expect(idx).toBeGreaterThanOrEqual(132);
      expect(heap.getObject(idx)).toBe(self);
    });

    it('should match _self_ pattern', () => {
      const fn = proxy['__wbg__self_abc123'] as () => number;
      expect(typeof fn).toBe('function');

      const idx = fn();
      expect(heap.getObject(idx)).toBe(self);
    });

    it('should match WINDOW accessor pattern', () => {
      const fn = proxy['__wbg_static_accessor_WINDOW_abc123'] as () => number;
      expect(typeof fn).toBe('function');

      // In test environment, window may or may not exist
      const idx = fn();
      if (typeof window !== 'undefined') {
        expect(idx).toBeGreaterThanOrEqual(132);
      } else {
        expect(idx).toBe(0);
      }
    });

    it('should match GLOBAL_THIS accessor pattern', () => {
      const fn = proxy['__wbg_static_accessor_GLOBAL_THIS_abc123'] as () => number;
      expect(typeof fn).toBe('function');

      const idx = fn();
      expect(heap.getObject(idx)).toBe(globalThis);
    });

    it('should match _globalThis_ pattern', () => {
      const fn = proxy['__wbg__globalThis_abc123'] as () => number;
      expect(typeof fn).toBe('function');

      const idx = fn();
      expect(heap.getObject(idx)).toBe(globalThis);
    });
  });

  describe('type checking handlers', () => {
    it('should match is_object pattern', () => {
      const fn = proxy['__wbg_is_object_abc123'] as (idx: number) => boolean;
      expect(typeof fn).toBe('function');

      const objIdx = heap.addObject({ test: true });
      const strIdx = heap.addObject('hello');
      const nullIdx = 129; // pre-populated null

      expect(fn(objIdx)).toBe(true);
      expect(fn(strIdx)).toBe(false);
      expect(fn(nullIdx)).toBe(false); // null is not an object in this check
    });

    it('should match is_undefined pattern', () => {
      const fn = proxy['__wbg_is_undefined_abc123'] as (idx: number) => boolean;
      expect(typeof fn).toBe('function');

      const undefinedIdx = 128;
      const objIdx = heap.addObject({});

      expect(fn(undefinedIdx)).toBe(true);
      expect(fn(objIdx)).toBe(false);
    });

    it('should match is_string pattern', () => {
      const fn = proxy['__wbg_is_string_abc123'] as (idx: number) => boolean;
      expect(typeof fn).toBe('function');

      const strIdx = heap.addObject('hello');
      const numIdx = heap.addObject(42);

      expect(fn(strIdx)).toBe(true);
      expect(fn(numIdx)).toBe(false);
    });

    it('should match is_function pattern', () => {
      const fn = proxy['__wbg_is_function_abc123'] as (idx: number) => boolean;
      expect(typeof fn).toBe('function');

      const fnIdx = heap.addObject(() => {});
      const objIdx = heap.addObject({});

      expect(fn(fnIdx)).toBe(true);
      expect(fn(objIdx)).toBe(false);
    });
  });

  describe('Uint8Array handlers', () => {
    it('should match new_with_length pattern', () => {
      const fn = proxy['__wbg_new_with_length_abc123'] as (len: number) => number;
      expect(typeof fn).toBe('function');

      const idx = fn(16);
      const arr = heap.getObject(idx) as Uint8Array;

      expect(arr).toBeInstanceOf(Uint8Array);
      expect(arr.length).toBe(16);
    });

    it('should match subarray pattern', () => {
      const fn = proxy['__wbg_subarray_abc123'] as (
        arrayIdx: number,
        start: number,
        end: number
      ) => number;
      expect(typeof fn).toBe('function');

      const original = new Uint8Array([1, 2, 3, 4, 5]);
      const arrayIdx = heap.addObject(original);

      const subIdx = fn(arrayIdx, 1, 4);
      const sub = heap.getObject(subIdx) as Uint8Array;

      expect(sub.length).toBe(3);
      expect(Array.from(sub)).toEqual([2, 3, 4]);
    });

    it('should match _length_ pattern', () => {
      const fn = proxy['__wbg__length_abc123'] as (arrayIdx: number) => number;
      expect(typeof fn).toBe('function');

      const arr = new Uint8Array(42);
      const idx = heap.addObject(arr);

      expect(fn(idx)).toBe(42);
    });

    it('should match prototypesetcall pattern', () => {
      const fn = proxy['__wbg_prototypesetcall_abc123'] as (
        wasmPtr: number,
        length: number,
        srcArrayIdx: number
      ) => void;
      expect(typeof fn).toBe('function');

      const srcData = new Uint8Array([10, 20, 30, 40]);
      const srcIdx = heap.addObject(srcData);
      const wasmPtr = 2048;

      fn(wasmPtr, srcData.length, srcIdx);

      const result = new Uint8Array(memory.buffer, wasmPtr, srcData.length);
      expect(Array.from(result)).toEqual([10, 20, 30, 40]);
    });

    it('should match set_ pattern for heap-to-heap copy', () => {
      const fn = proxy['__wbg_set_abc123'] as (
        targetIdx: number,
        srcIdx: number,
        offset?: number
      ) => void;
      expect(typeof fn).toBe('function');

      const target = new Uint8Array(10);
      const src = new Uint8Array([1, 2, 3]);
      const targetIdx = heap.addObject(target);
      const srcIdx = heap.addObject(src);

      fn(targetIdx, srcIdx, 2);

      expect(Array.from(target)).toEqual([0, 0, 1, 2, 3, 0, 0, 0, 0, 0]);
    });

    it('should match copy_to pattern', () => {
      const fn = proxy['__wbg_copy_to_abc123'] as (arrayIdx: number, ptr: number) => void;
      expect(typeof fn).toBe('function');

      const arr = new Uint8Array([5, 6, 7, 8]);
      const idx = heap.addObject(arr);
      const ptr = 3072;

      fn(idx, ptr);

      const result = new Uint8Array(memory.buffer, ptr, arr.length);
      expect(Array.from(result)).toEqual([5, 6, 7, 8]);
    });
  });

  describe('Node.js detection handlers', () => {
    it('should return undefined for _process_ pattern', () => {
      const fn = proxy['__wbg__process_abc123'] as () => undefined;
      expect(typeof fn).toBe('function');
      expect(fn()).toBeUndefined();
    });

    it('should return undefined for _versions_ pattern', () => {
      const fn = proxy['__wbg__versions_abc123'] as () => undefined;
      expect(fn()).toBeUndefined();
    });

    it('should return undefined for _node_ pattern', () => {
      const fn = proxy['__wbg__node_abc123'] as () => undefined;
      expect(fn()).toBeUndefined();
    });

    it('should return undefined for _require_ pattern', () => {
      const fn = proxy['__wbg__require_abc123'] as () => undefined;
      expect(fn()).toBeUndefined();
    });

    it('should return undefined for msCrypto pattern', () => {
      const fn = proxy['__wbg_msCrypto_abc123'] as () => undefined;
      expect(fn()).toBeUndefined();
    });
  });

  describe('new_no_args handler', () => {
    it('should create function from string', () => {
      const fn = proxy['__wbg_new_no_args_abc123'] as (ptr: number, len: number) => number;
      expect(typeof fn).toBe('function');

      const code = 'return 42';
      const ptr = 1024;
      const len = writeString(memory, ptr, code);

      const idx = fn(ptr, len);
      const createdFn = heap.getObject(idx) as () => number;

      expect(typeof createdFn).toBe('function');
      expect(createdFn()).toBe(42);
    });
  });

  describe('Function.prototype.call handler', () => {
    it('should call function with args from heap', () => {
      const fn = proxy['__wbg__call_abc123'] as (
        fnIdx: number,
        thisArgIdx: number,
        ...argIdxs: number[]
      ) => unknown;
      expect(typeof fn).toBe('function');

      const testFn = function (this: { value: number }, x: number) {
        return this.value + x;
      };
      const thisArg = { value: 10 };

      const fnIdx = heap.addObject(testFn);
      const thisIdx = heap.addObject(thisArg);
      const argIdx = heap.addObject(5);

      const result = fn(fnIdx, thisIdx, argIdx);
      expect(result).toBe(15);
    });
  });

  describe('unknown import fallback', () => {
    it('should return function for unknown imports', () => {
      const fn = proxy['__wbg_completely_unknown_import'];
      expect(typeof fn).toBe('function');
    });

    it('should log warning when unknown import is called', () => {
      const warnSpy = vi.spyOn(console, 'warn');
      const fn = proxy['__wbg_unknown_xyz'] as (...args: unknown[]) => unknown;

      fn(1, 2, 3);

      expect(warnSpy).toHaveBeenCalledWith(
        expect.stringContaining('UNKNOWN wasm-bindgen import: __wbg_unknown_xyz')
      );
    });

    it('should return undefined from unknown imports', () => {
      const fn = proxy['__wbg_unknown'] as () => unknown;
      expect(fn()).toBeUndefined();
    });
  });
});

describe('createExternrefShims', () => {
  it('should create a table with correct size', () => {
    const { table } = createExternrefShims();

    expect(table).toBeInstanceOf(WebAssembly.Table);
    expect(table.length).toBe(128);
  });

  describe('__wbindgen_externref_table_grow', () => {
    it('should return table length', () => {
      const { table, shims } = createExternrefShims();
      const grow = shims.__wbindgen_externref_table_grow as (delta: number) => number;

      expect(grow(0)).toBe(table.length);
    });
  });

  describe('__wbindgen_externref_table_set_null', () => {
    it('should set table entry to null', () => {
      const { table, shims } = createExternrefShims();
      const setNull = shims.__wbindgen_externref_table_set_null as (idx: number) => void;

      // First set a value
      const testObj = { test: true };
      table.set(5, testObj);
      expect(table.get(5)).toBe(testObj);

      // Then set to null
      setNull(5);
      expect(table.get(5)).toBe(null);
    });

    it('should not throw for out-of-bounds index', () => {
      const { shims } = createExternrefShims();
      const setNull = shims.__wbindgen_externref_table_set_null as (idx: number) => void;

      // Should not throw
      expect(() => setNull(999)).not.toThrow();
    });
  });
});

describe('createExternrefProxy', () => {
  let shims: Record<string, (...args: unknown[]) => unknown>;
  let proxy: Record<string, (...args: unknown[]) => unknown>;
  const workerId = 1;

  beforeEach(() => {
    const externref = createExternrefShims();
    shims = externref.shims;
    const handler = createExternrefProxy(shims, workerId);
    proxy = new Proxy(shims, handler);
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return existing shim functions', () => {
    expect(typeof proxy.__wbindgen_externref_table_grow).toBe('function');
    expect(typeof proxy.__wbindgen_externref_table_set_null).toBe('function');
  });

  it('should return function for unknown externref imports', () => {
    const fn = proxy.__wbindgen_externref_unknown;
    expect(typeof fn).toBe('function');
  });

  it('should return undefined from unknown externref imports', () => {
    const fn = proxy.__wbindgen_externref_xyz as () => unknown;
    expect(fn()).toBeUndefined();
  });

  it('should log when shimming unknown import', () => {
    const logSpy = vi.spyOn(console, 'log');
    proxy.__wbindgen_externref_new_import;

    expect(logSpy).toHaveBeenCalledWith(
      expect.stringContaining('Shimming unknown externref import')
    );
  });
});
