/**
 * Zero OS Process Worker - TypeScript Entry Point
 *
 * This script is a thin shim that:
 * 1. Instantiates the WASM binary with syscall imports
 * 2. Uses the WASM module's memory for the syscall mailbox
 * 3. Reports memory to supervisor for syscall polling
 * 4. Calls _start() - WASM runs forever using atomics-based syscalls
 *
 * Mailbox Layout (at offset 0 in WASM linear memory):
 * | Offset | Size | Field                              |
 * |--------|------|------------------------------------|
 * | 0      | 4    | status (0=idle, 1=pending, 2=ready)|
 * | 4      | 4    | syscall_num                        |
 * | 8      | 4    | arg0                               |
 * | 12     | 4    | arg1                               |
 * | 16     | 4    | arg2                               |
 * | 20     | 4    | result                             |
 * | 24     | 4    | data_len                           |
 * | 28     | 16356| data buffer (16KB total buffer)    |
 * | 56     | 4    | pid (stored by supervisor)         |
 */

import {
  MAILBOX_OFFSETS,
  createWorkerState,
  type WorkerState,
  type WasmExports,
  type SupervisorMessage,
} from './types';
import {
  zos_syscall,
  zos_send_bytes,
  zos_recv_bytes,
  zos_yield,
  zos_get_pid,
} from './mailbox';
import { WasmBindgenHeap } from './heap';
import {
  createWasmBindgenShims,
  createExternrefShims,
  createWasmBindgenProxy,
  createExternrefProxy,
} from './wasm-bindgen-shims';

declare const self: DedicatedWorkerGlobalScope;

// Capture the worker's memory context ID from the browser
// performance.timeOrigin is the Unix timestamp (ms) when this worker context was created
const WORKER_MEMORY_ID = Math.floor(performance.timeOrigin);

// Worker state (set after initialization)
const state: WorkerState = createWorkerState(WORKER_MEMORY_ID);

/**
 * Post memory buffer to supervisor
 */
function postMemoryUpdate(): void {
  if (!state.wasmMemory) return;
  const msg: SupervisorMessage = {
    type: 'memory',
    pid: state.pid,
    buffer: state.wasmMemory.buffer as SharedArrayBuffer,
    workerId: state.workerId,
  };
  self.postMessage(msg);
}

/**
 * Create bound syscall functions that capture state
 */
function createSyscallImports() {
  return {
    zos_syscall: (
      syscall_num: number,
      arg0: number,
      arg1: number,
      arg2: number
    ): number => zos_syscall(state, postMemoryUpdate, syscall_num, arg0, arg1, arg2),
    zos_send_bytes: (ptr: number, len: number): number =>
      zos_send_bytes(state, postMemoryUpdate, ptr, len),
    zos_recv_bytes: (ptr: number, maxLen: number): number =>
      zos_recv_bytes(state, postMemoryUpdate, ptr, maxLen),
    zos_yield: (): void => zos_yield(state, postMemoryUpdate),
    zos_get_pid: (): number => zos_get_pid(state, postMemoryUpdate),
  };
}

/**
 * Main message handler
 */
self.onmessage = async (event: MessageEvent) => {
  const data = event.data;

  // Handle terminate message
  if (data.type === 'terminate') {
    self.close();
    return;
  }

  // IPC messages are handled via syscalls - ignore
  if (data.type === 'ipc') {
    return;
  }

  // If already initialized, ignore unknown messages
  if (state.initialized) {
    console.log(
      `[worker:${WORKER_MEMORY_ID}] Ignoring message after init:`,
      data.type || 'unknown'
    );
    return;
  }

  // Initial spawn message with WASM binary
  const { binary, pid } = data;

  if (!binary || !pid) {
    console.error(
      `[worker:${WORKER_MEMORY_ID}] Invalid init message - missing binary or pid`
    );
    return;
  }

  state.pid = pid;

  try {
    // Compile the WASM module to inspect its imports/exports
    const module = await WebAssembly.compile(binary);

    // Check what the module needs
    const imports = WebAssembly.Module.imports(module);
    const exports = WebAssembly.Module.exports(module);
    const importsMemory = imports.some(
      (imp) =>
        imp.module === 'env' && imp.name === 'memory' && imp.kind === 'memory'
    );
    const exportsMemory = exports.some(
      (exp) => exp.name === 'memory' && exp.kind === 'memory'
    );

    console.log(
      `[worker:${WORKER_MEMORY_ID}] Module imports memory: ${importsMemory}, exports memory: ${exportsMemory}`
    );

    // Create heap for wasm-bindgen
    const heap = new WasmBindgenHeap();

    // Create wasm-bindgen shims
    const getMemory = () => state.wasmMemory;
    const baseShims = createWasmBindgenShims(
      heap,
      getMemory,
      WORKER_MEMORY_ID,
      pid
    );
    const bindgenProxy = createWasmBindgenProxy(
      baseShims,
      heap,
      getMemory,
      WORKER_MEMORY_ID,
      pid
    );

    // Create externref shims
    const { shims: externrefShims } = createExternrefShims();
    const externrefProxy = createExternrefProxy(externrefShims, WORKER_MEMORY_ID);

    // Build import object
    let sharedMemory: WebAssembly.Memory | null = null;
    const importObject: WebAssembly.Imports = {
      env: {
        ...createSyscallImports(),
      },
      __wbindgen_externref_xform__: new Proxy(externrefShims, externrefProxy),
      __wbindgen_placeholder__: new Proxy(baseShims, bindgenProxy),
    };

    // If module imports memory, provide shared memory
    if (importsMemory) {
      sharedMemory = new WebAssembly.Memory({
        initial: 32, // 2MB initial (32 * 64KB)
        maximum: 64, // 4MB max (64 * 64KB)
        shared: true,
      });
      (importObject.env as Record<string, unknown>).memory = sharedMemory;
    }

    // Instantiate the module
    const instance = await WebAssembly.instantiate(module, importObject);
    const wasmExports = instance.exports as unknown as WasmExports;

    // Determine which memory to use
    if (importsMemory && sharedMemory) {
      state.wasmMemory = sharedMemory;
      console.log(`[worker:${WORKER_MEMORY_ID}] Using imported shared memory`);
    } else if (exportsMemory && wasmExports.memory) {
      state.wasmMemory = wasmExports.memory;
      console.log(`[worker:${WORKER_MEMORY_ID}] Using module's exported memory`);

      if (!(state.wasmMemory.buffer instanceof SharedArrayBuffer)) {
        console.warn(
          `[worker:${WORKER_MEMORY_ID}] Module memory is not shared - atomics may not work correctly`
        );
      }
    } else {
      throw new Error('WASM module has no accessible memory');
    }

    // Create typed array views for the mailbox
    state.mailboxView = new Int32Array(state.wasmMemory.buffer);
    state.mailboxBytes = new Uint8Array(state.wasmMemory.buffer);

    // Store PID in mailbox
    Atomics.store(state.mailboxView, MAILBOX_OFFSETS.PID, pid);

    // Debug logging for init process
    if (pid === 1) {
      console.log(
        `[worker:${WORKER_MEMORY_ID}] Init (PID 1) sending SharedArrayBuffer to supervisor`
      );
      console.log(
        `[worker:${WORKER_MEMORY_ID}] Buffer is SharedArrayBuffer: ${state.wasmMemory.buffer instanceof SharedArrayBuffer}`
      );
      console.log(
        `[worker:${WORKER_MEMORY_ID}] Buffer size: ${state.wasmMemory.buffer.byteLength} bytes`
      );

      // Write a magic marker at a known offset to verify buffer identity
      const markerView = new Uint32Array(state.wasmMemory.buffer);
      markerView[4096] = 0xdeadbeef;
      console.log(
        `[worker:${WORKER_MEMORY_ID}:${pid}] Wrote buffer marker 0xDEADBEEF at offset 16384`
      );
    }

    // Send memory buffer to supervisor
    postMemoryUpdate();

    // Mark as initialized before running
    state.initialized = true;

    // Initialize runtime if the module exports it
    if (wasmExports.__zero_rt_init) {
      console.log(`[worker:${WORKER_MEMORY_ID}:${pid}] Calling __zero_rt_init`);
      wasmExports.__zero_rt_init(0);
    }

    // Run the process - blocks forever using atomics-based syscalls
    if (wasmExports._start) {
      console.log(
        `[worker:${WORKER_MEMORY_ID}:${pid}] Calling _start() - process will block on atomics`
      );
      wasmExports._start();
      console.log(
        `[worker:${WORKER_MEMORY_ID}:${pid}] _start() returned (should never happen)`
      );
    } else {
      console.error(`[worker:${WORKER_MEMORY_ID}:${pid}] No _start export found!`);
    }
  } catch (e) {
    const error = e as Error;
    console.error(`[worker:${WORKER_MEMORY_ID}] Error:`, error);
    const msg: SupervisorMessage = {
      type: 'error',
      pid: state.pid,
      workerId: WORKER_MEMORY_ID,
      error: error.message,
    };
    self.postMessage(msg);
  }
};
