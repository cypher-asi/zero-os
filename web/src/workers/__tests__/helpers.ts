/**
 * Test Helpers for Worker Tests
 *
 * Provides utilities for creating test fixtures and writing data to WASM memory.
 */

import { type WorkerState } from '../types';

/**
 * Create a WorkerState with real SharedArrayBuffer memory
 * for integration tests requiring WASM-like memory operations.
 */
export function createTestState(pid = 1, workerId = 12345): WorkerState {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true });
  return {
    initialized: true,
    pid,
    workerId,
    wasmMemory: memory,
    mailboxView: new Int32Array(memory.buffer),
    mailboxBytes: new Uint8Array(memory.buffer),
  };
}

/**
 * Create a WorkerState with regular (non-shared) ArrayBuffer memory.
 * Used for testing non-SharedArrayBuffer code paths.
 */
export function createTestStateNonShared(pid = 1, workerId = 12345): WorkerState {
  const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
  return {
    initialized: true,
    pid,
    workerId,
    wasmMemory: memory,
    mailboxView: new Int32Array(memory.buffer),
    mailboxBytes: new Uint8Array(memory.buffer),
  };
}

/**
 * Create an uninitialized WorkerState (no memory).
 * Used for testing error handling paths.
 */
export function createUninitializedState(workerId = 12345): WorkerState {
  return {
    initialized: false,
    pid: 0,
    workerId,
    wasmMemory: null,
    mailboxView: null,
    mailboxBytes: null,
  };
}

/**
 * Write a string into WASM memory at the specified pointer.
 * Returns the number of bytes written (UTF-8 encoded length).
 */
export function writeString(memory: WebAssembly.Memory, ptr: number, str: string): number {
  const encoded = new TextEncoder().encode(str);
  const view = new Uint8Array(memory.buffer, ptr, encoded.length);
  view.set(encoded);
  return encoded.length;
}

/**
 * Write raw bytes into WASM memory at the specified pointer.
 */
export function writeBytes(memory: WebAssembly.Memory, ptr: number, bytes: Uint8Array): void {
  const view = new Uint8Array(memory.buffer, ptr, bytes.length);
  view.set(bytes);
}

/**
 * Read bytes from WASM memory at the specified pointer.
 */
export function readBytes(memory: WebAssembly.Memory, ptr: number, len: number): Uint8Array {
  return new Uint8Array(memory.buffer, ptr, len).slice();
}

/**
 * Read a UTF-8 string from WASM memory at the specified pointer.
 */
export function readString(memory: WebAssembly.Memory, ptr: number, len: number): string {
  const bytes = readBytes(memory, ptr, len);
  return new TextDecoder().decode(bytes);
}

/**
 * Create a mock postMemoryUpdate function that tracks calls.
 */
export function createMockPostMemoryUpdate(): { fn: () => void; calls: number } {
  const tracker = { fn: () => {}, calls: 0 };
  tracker.fn = () => {
    tracker.calls++;
  };
  return tracker;
}
