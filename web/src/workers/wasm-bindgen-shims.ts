/**
 * Zero OS Worker - wasm-bindgen Compatibility Shims
 *
 * Provides all the shims needed for wasm-bindgen imports, including
 * getrandom/crypto support, object management, and various JS interop functions.
 */

import { WasmBindgenHeap } from './heap';
import { getRandomValues, fillRandomArray } from './crypto';

declare const self: DedicatedWorkerGlobalScope;

/**
 * Helper to decode string from WASM memory (handles SharedArrayBuffer)
 * TextDecoder.decode() doesn't support SharedArrayBuffer views, so we copy first
 */
export function decodeWasmString(
  memory: WebAssembly.Memory | null,
  ptr: number,
  len: number
): string {
  if (!memory) return '';
  const sharedView = new Uint8Array(memory.buffer, ptr, len);
  // Copy to non-shared buffer for TextDecoder compatibility
  const copied = new Uint8Array(len);
  copied.set(sharedView);
  return new TextDecoder().decode(copied);
}

/**
 * Creates the base wasm-bindgen shim functions
 */
export function createWasmBindgenShims(
  heap: WasmBindgenHeap,
  getMemory: () => WebAssembly.Memory | null,
  workerId: number,
  pid: number
): Record<string, (...args: unknown[]) => unknown> {
  const decode = (ptr: number, len: number) =>
    decodeWasmString(getMemory(), ptr, len);

  return {
    __wbindgen_throw(ptr: number, len: number): never {
      const memory = getMemory();
      if (!memory) {
        throw new Error('wasm-bindgen error: memory not initialized');
      }
      const msg = decode(ptr, len);
      throw new Error(msg);
    },

    __wbindgen_rethrow(arg: unknown): never {
      throw arg;
    },

    __wbindgen_memory(): WebAssembly.Memory | null {
      return getMemory();
    },

    __wbindgen_is_undefined(idx: number): boolean {
      return heap.getObject(idx) === undefined;
    },

    __wbindgen_is_null(idx: number): boolean {
      return heap.getObject(idx) === null;
    },

    __wbindgen_object_drop_ref(idx: number): void {
      heap.dropObject(idx);
    },

    __wbindgen_object_clone_ref(idx: number): number {
      return heap.addObject(heap.getObject(idx));
    },

    __wbindgen_string_new(ptr: number, len: number): number {
      return heap.addObject(decode(ptr, len));
    },

    __wbindgen_string_get(val: unknown, retptr: number): void {
      const memory = getMemory();
      if (!memory || typeof val !== 'string') {
        const view = new DataView(memory!.buffer);
        view.setUint32(retptr, 0, true);
        view.setUint32(retptr + 4, 0, true);
        return;
      }
      const encoded = new TextEncoder().encode(val);
      const view = new DataView(memory.buffer);
      view.setUint32(retptr, encoded.byteOffset, true);
      view.setUint32(retptr + 4, encoded.byteLength, true);
    },

    __wbindgen_number_get(val: unknown): number | undefined {
      return typeof val === 'number' ? val : undefined;
    },

    __wbindgen_boolean_get(val: unknown): boolean | undefined {
      return typeof val === 'boolean' ? val : undefined;
    },

    __wbindgen_jsval_eq(a: unknown, b: unknown): boolean {
      return a === b;
    },

    __wbindgen_describe(_val: number): void {
      // Type description function - no-op for our use case
    },

    __wbindgen_error_new(ptr: number, len: number): number {
      const msg = decode(ptr, len) || 'Unknown error';
      return heap.addObject(new Error(msg));
    },

    __wbindgen_cb_drop(_idx: number): boolean {
      return true;
    },

    __wbindgen_describe_cast(): void {
      // Type cast description - no-op
    },

    // Mangled version of __wbindgen_throw with hash suffix
    __wbg___wbindgen_throw_be289d5034ed271b(ptr: number, len: number): never {
      const memory = getMemory();
      if (!memory) {
        throw new Error('wasm-bindgen error: memory not initialized');
      }
      const msg = decode(ptr, len);
      throw new Error(msg);
    },

    // Crypto API for getrandom
    __wbindgen_get_random_values(ptr: number, len: number): void {
      const memory = getMemory();
      if (!memory) {
        throw new Error('Memory not initialized for getRandomValues');
      }
      getRandomValues(memory, ptr, len, workerId, pid);
    },
  };
}

/**
 * Creates the externref table and its shim functions
 */
export function createExternrefShims(): {
  table: WebAssembly.Table;
  shims: Record<string, (...args: unknown[]) => unknown>;
} {
  const table = new WebAssembly.Table({
    initial: 128,
    maximum: 128,
    element: 'externref',
  });

  return {
    table,
    shims: {
      __wbindgen_externref_table_grow(_delta: number): number {
        return table.length;
      },
      __wbindgen_externref_table_set_null(idx: number): void {
        if (idx < table.length) {
          table.set(idx, null);
        }
      },
    },
  };
}

/**
 * Creates a Proxy handler for the wasm-bindgen placeholder module.
 * This handles dynamic imports that match various patterns for crypto,
 * global accessors, typed arrays, etc.
 */
export function createWasmBindgenProxy(
  baseShims: Record<string, (...args: unknown[]) => unknown>,
  heap: WasmBindgenHeap,
  getMemory: () => WebAssembly.Memory | null,
  workerId: number,
  pid: number
): ProxyHandler<Record<string, (...args: unknown[]) => unknown>> {
  const decode = (ptr: number, len: number) =>
    decodeWasmString(getMemory(), ptr, len);

  const log = (msg: string) =>
    console.log(`[worker:${workerId}:${pid}] ${msg}`);
  const warn = (msg: string) =>
    console.warn(`[worker:${workerId}:${pid}] ${msg}`);
  const error = (msg: string) =>
    console.error(`[worker:${workerId}:${pid}] ${msg}`);

  return {
    get(
      target: Record<string, (...args: unknown[]) => unknown>,
      prop: string | symbol
    ): unknown {
      if (typeof prop !== 'string') return undefined;

      // Return existing function if available
      if (prop in target) {
        return target[prop];
      }

      // === Crypto API handlers ===
      if (prop.includes('crypto') && !prop.includes('msCrypto')) {
        log(`PROXY: crypto getter shim matched for import: ${prop}`);
        return function (selfIdx: number): number {
          const selfObj = heap.getObject(selfIdx) as {
            crypto?: Crypto;
          } | null;
          const crypto = selfObj?.crypto || self.crypto;
          log(
            `PROXY crypto getter: selfIdx=${selfIdx}, selfObj type=${typeof selfObj}, found crypto=${!!crypto}`
          );
          if (!crypto) {
            error(`PROXY crypto getter: FAILED - no crypto object found!`);
          }
          return heap.addObject(crypto);
        };
      }

      if (prop.includes('getRandomValues')) {
        log(`PROXY: getRandomValues shim matched for import: ${prop}`);
        return function (cryptoIdx: number, arrayIdx: number): void {
          const cryptoObj =
            (heap.getObject(cryptoIdx) as Crypto) || self.crypto;
          const array = heap.getObject(arrayIdx) as Uint8Array;

          log(
            `PROXY getRandomValues: cryptoIdx=${cryptoIdx}, arrayIdx=${arrayIdx}, arrayLen=${array?.length}`
          );

          if (array instanceof Uint8Array) {
            fillRandomArray(array, workerId, pid);
          } else {
            error(
              `PROXY getRandomValues: expected Uint8Array, got ${typeof array}`
            );
            throw new Error(
              `getRandomValues: expected Uint8Array, got ${typeof array}`
            );
          }
        };
      }

      if (prop.includes('randomFillSync')) {
        return function (_cryptoObj: unknown, ptr: number, len: number): void {
          const memory = getMemory();
          if (!memory) {
            throw new Error('Memory not initialized for randomFillSync');
          }
          const buf = new Uint8Array(memory.buffer, ptr, len);
          if (memory.buffer instanceof SharedArrayBuffer) {
            const copy = new Uint8Array(len);
            self.crypto.getRandomValues(copy);
            buf.set(copy);
          } else {
            self.crypto.getRandomValues(buf);
          }
        };
      }

      // === Global accessor handlers ===
      if (prop.includes('static_accessor_SELF') || prop.includes('_self_')) {
        log(`PROXY: SELF accessor shim matched for import: ${prop}`);
        return function (): number {
          log(`PROXY SELF(): returning self (Web Worker global)`);
          return heap.addObject(self);
        };
      }

      if (
        prop.includes('static_accessor_WINDOW') ||
        prop.includes('_window_')
      ) {
        log(`PROXY: WINDOW accessor shim matched for import: ${prop}`);
        return function (): number {
          const obj = typeof window !== 'undefined' ? window : undefined;
          log(
            `PROXY WINDOW(): window exists=${typeof window !== 'undefined'}, returning ${obj === undefined ? '0 (undefined)' : 'window object'}`
          );
          if (obj === undefined) return 0;
          return heap.addObject(obj);
        };
      }

      if (
        prop.includes('static_accessor_GLOBAL_THIS') ||
        prop.includes('_globalThis_')
      ) {
        log(`PROXY: GLOBAL_THIS accessor shim matched for import: ${prop}`);
        return function (): number {
          log(
            `PROXY GLOBAL_THIS(): returning globalThis (has crypto=${!!globalThis.crypto})`
          );
          return heap.addObject(globalThis);
        };
      }

      if (
        prop.includes('static_accessor_GLOBAL') ||
        prop.includes('_global_')
      ) {
        log(`PROXY: GLOBAL accessor shim matched for import: ${prop}`);
        return function (): number {
          const obj =
            typeof global !== 'undefined' ? global : (self as unknown);
          log(
            `PROXY GLOBAL(): global exists=${typeof global !== 'undefined'}, returning ${typeof global !== 'undefined' ? 'global' : 'self (fallback)'}`
          );
          return heap.addObject(obj);
        };
      }

      // === Type checking handlers ===
      if (prop.includes('new_no_args') || prop.includes('newnoargs')) {
        return function (ptr: number, len: number): number {
          const str = decode(ptr, len);
          const fn = new Function(str);
          return heap.addObject(fn);
        };
      }

      if (prop.includes('is_object')) {
        return function (idx: number): boolean {
          const val = heap.getObject(idx);
          return typeof val === 'object' && val !== null;
        };
      }

      if (prop.includes('is_undefined')) {
        return function (idx: number): boolean {
          return heap.getObject(idx) === undefined;
        };
      }

      if (prop.includes('is_string')) {
        return function (idx: number): boolean {
          return typeof heap.getObject(idx) === 'string';
        };
      }

      if (prop.includes('is_function')) {
        return function (idx: number): boolean {
          return typeof heap.getObject(idx) === 'function';
        };
      }

      // === Uint8Array handlers ===
      if (prop.includes('new_with_length') || prop.includes('newwithlength')) {
        log(`PROXY: new_with_length shim matched for import: ${prop}`);
        return function (len: number): number {
          const array = new Uint8Array(len);
          const idx = heap.addObject(array);
          log(`PROXY Uint8Array(${len}): created, heap idx=${idx}`);
          return idx;
        };
      }

      if (prop.includes('subarray')) {
        log(`PROXY: subarray shim matched for import: ${prop}`);
        return function (arrayIdx: number, start: number, end: number): number {
          const array = heap.getObject(arrayIdx) as Uint8Array;
          const subarray = array.subarray(start, end);
          const idx = heap.addObject(subarray);
          log(
            `PROXY subarray: arrayIdx=${arrayIdx}, [${start}:${end}], len=${subarray.length}, heap idx=${idx}`
          );
          return idx;
        };
      }

      if (prop.includes('_length_')) {
        return function (arrayIdx: number): number {
          const array = heap.getObject(arrayIdx) as Uint8Array | null;
          return array?.length || 0;
        };
      }

      // Uint8Array.prototype.set.call() - used by wasm-bindgen for raw_copy_to_ptr
      if (prop.includes('prototypesetcall')) {
        log(`PROXY: prototypesetcall shim matched for import: ${prop}`);
        return function (
          wasmPtr: number,
          length: number,
          srcArrayIdx: number
        ): void {
          const srcArray = heap.getObject(srcArrayIdx) as Uint8Array;
          log(
            `PROXY prototypesetcall: wasmPtr=${wasmPtr}, length=${length}, srcArrayIdx=${srcArrayIdx}, srcArray type=${srcArray?.constructor?.name}`
          );

          const memory = getMemory();
          if (!memory || !memory.buffer) {
            error(`PROXY prototypesetcall: FAILED - wasmMemory not available!`);
            return;
          }

          if (!(srcArray instanceof Uint8Array)) {
            error(
              `PROXY prototypesetcall: FAILED - srcArray is not Uint8Array!`
            );
            return;
          }

          const dest = new Uint8Array(memory.buffer, wasmPtr, length);
          dest.set(srcArray.subarray(0, length));

          const preview = Array.from(srcArray.slice(0, Math.min(8, length)))
            .map((b) => b.toString(16).padStart(2, '0'))
            .join('');
          log(
            `PROXY prototypesetcall: SUCCESS - copied ${length} bytes to WASM ptr ${wasmPtr}, preview: ${preview}...`
          );
        };
      }

      // Uint8Array.prototype.set - copy data from one array to another
      if (prop.includes('set_') || prop.includes('_set_')) {
        log(`PROXY: set shim matched for import: ${prop}`);
        return function (
          arg0: number,
          arg1: number,
          arg2: number | undefined
        ): void {
          const src = heap.getObject(arg0) as Uint8Array;

          // Detect if arg1 is a raw WASM pointer or a heap index
          if (
            src instanceof Uint8Array &&
            typeof arg1 === 'number' &&
            arg2 === undefined
          ) {
            if (arg1 > 200 || !heap.getObject(arg1)) {
              // This is raw_copy_to_ptr: copy JS array to WASM memory
              log(
                `PROXY set (raw_copy_to_ptr): copying ${src.length} bytes from JS array to WASM ptr ${arg1}`
              );

              const memory = getMemory();
              if (!memory || !memory.buffer) {
                error(`PROXY set: FAILED - wasmMemory not available!`);
                return;
              }

              const dest = new Uint8Array(memory.buffer, arg1, src.length);
              dest.set(src);

              const preview = Array.from(src.slice(0, Math.min(8, src.length)))
                .map((b) => b.toString(16).padStart(2, '0'))
                .join('');
              log(
                `PROXY set: copied to WASM memory, preview: ${preview}...`
              );
              return;
            }
          }

          // Standard case: both args are heap objects
          const target = heap.getObject(arg0) as Uint8Array;
          const srcObj = heap.getObject(arg1) as Uint8Array;
          log(
            `PROXY set (standard): targetIdx=${arg0}, srcIdx=${arg1}, offset=${arg2}, srcLen=${srcObj?.length}`
          );

          if (arg2 !== undefined) {
            target.set(srcObj, arg2);
          } else {
            target.set(srcObj);
          }

          if (srcObj && srcObj.length > 0) {
            const preview = Array.from(
              srcObj.slice(0, Math.min(8, srcObj.length))
            )
              .map((b) => b.toString(16).padStart(2, '0'))
              .join('');
            log(`PROXY set data preview: ${preview}...`);
          }
        };
      }

      // Copy typed array to WASM memory (raw_copy_to_ptr)
      if (
        prop.includes('copy_to') ||
        prop.includes('copyto') ||
        prop.includes('raw_copy')
      ) {
        log(`PROXY: copy_to shim matched for import: ${prop}`);
        return function (arrayIdx: number, ptr: number): void {
          const array = heap.getObject(arrayIdx) as Uint8Array;
          log(
            `PROXY copy_to: arrayIdx=${arrayIdx}, ptr=${ptr}, len=${array?.length}`
          );

          const memory = getMemory();
          if (!memory || !memory.buffer) {
            error(`PROXY copy_to: FAILED - wasmMemory not available!`);
            return;
          }

          const dest = new Uint8Array(memory.buffer, ptr, array.length);
          dest.set(array);

          const preview = Array.from(array.slice(0, Math.min(8, array.length)))
            .map((b) => b.toString(16).padStart(2, '0'))
            .join('');
          log(
            `PROXY copy_to WASM memory: ${preview}... (${array.length} bytes to ptr ${ptr})`
          );
        };
      }

      // === Node.js detection - return undefined ===
      if (
        prop.includes('_process_') ||
        prop.includes('_versions_') ||
        prop.includes('_node_') ||
        prop.includes('_require_')
      ) {
        return function (): undefined {
          return undefined;
        };
      }

      // msCrypto - IE fallback, return undefined
      if (prop.includes('msCrypto')) {
        return function (): undefined {
          return undefined;
        };
      }

      // Function.prototype.call
      if (prop.includes('_call_')) {
        return function (
          fnIdx: number,
          thisArgIdx: number,
          ...argIdxs: number[]
        ): number | unknown {
          const fn = heap.getObject(fnIdx) as (
            ...args: unknown[]
          ) => unknown | undefined;
          const thisArg = heap.getObject(thisArgIdx);
          const args = argIdxs.map((idx) => heap.getObject(idx));
          if (typeof fn === 'function') {
            const result = fn.call(thisArg, ...args);
            if (
              result !== undefined &&
              result !== null &&
              typeof result === 'object'
            ) {
              return heap.addObject(result);
            }
            return result;
          }
          return 0;
        };
      }

      if (prop === '__wbindgen_is_object') {
        return function (val: unknown): boolean {
          return typeof val === 'object' && val !== null;
        };
      }

      if (prop === '__wbindgen_is_function') {
        return function (val: unknown): boolean {
          return typeof val === 'function';
        };
      }

      // === Unknown import fallback ===
      warn(`UNKNOWN wasm-bindgen import: ${prop}`);
      return function (...args: unknown[]): undefined {
        const argTypes = args.map((a, i) => {
          if (typeof a === 'number') {
            const heapObj = heap.getObject(a);
            if (heapObj !== undefined) {
              return `arg${i}=${a} (heap: ${(heapObj as object)?.constructor?.name || typeof heapObj})`;
            }
            return `arg${i}=${a} (number, possibly ptr)`;
          }
          return `arg${i}=${typeof a}`;
        });
        warn(
          `Called unknown import '${prop}' with args: [${argTypes.join(', ')}]`
        );
        return undefined;
      };
    },
  };
}

/**
 * Creates a Proxy handler for the externref transform module
 */
export function createExternrefProxy(
  shims: Record<string, (...args: unknown[]) => unknown>,
  workerId: number
): ProxyHandler<Record<string, (...args: unknown[]) => unknown>> {
  return {
    get(
      target: Record<string, (...args: unknown[]) => unknown>,
      prop: string | symbol
    ): unknown {
      if (typeof prop !== 'string') return undefined;
      if (prop in target) {
        return target[prop];
      }
      console.log(
        `[worker:${workerId}] Shimming unknown externref import: ${prop}`
      );
      return function (): undefined {
        return undefined;
      };
    },
  };
}
