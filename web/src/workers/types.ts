/**
 * Zero OS Worker Types
 *
 * Shared type definitions, mailbox constants, and offsets for the WASM process worker.
 */

// Mailbox status values
export const STATUS_IDLE = 0;
export const STATUS_PENDING = 1;
export const STATUS_READY = 2;

// Mailbox field offsets (in i32 units)
export const MAILBOX_OFFSETS = {
  STATUS: 0,
  SYSCALL_NUM: 1,
  ARG0: 2,
  ARG1: 3,
  ARG2: 4,
  RESULT: 5,
  DATA_LEN: 6,
  DATA: 7, // Byte offset 28
  PID: 14, // PID storage location
} as const;

// Data buffer constants
export const MAILBOX_DATA_BYTE_OFFSET = 28;
export const MAILBOX_MAX_DATA_LEN = 16356;

/**
 * Message sent from supervisor to spawn a new process
 */
export interface SpawnMessage {
  binary: ArrayBuffer;
  pid: number;
}

/**
 * Message types the worker can receive
 */
export type WorkerMessage =
  | SpawnMessage
  | { type: 'terminate' }
  | { type: 'ipc'; [key: string]: unknown };

/**
 * Worker state maintained during process execution
 */
export interface WorkerState {
  initialized: boolean;
  pid: number;
  workerId: number;
  wasmMemory: WebAssembly.Memory | null;
  mailboxView: Int32Array | null;
  mailboxBytes: Uint8Array | null;
}

/**
 * Creates a fresh worker state object
 */
export function createWorkerState(workerId: number): WorkerState {
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
 * WASM module exports we care about
 */
export interface WasmExports {
  memory?: WebAssembly.Memory;
  _start?: () => void;
  __zero_rt_init?: (arg: number) => void;
}

/**
 * Message posted back to supervisor
 */
export type SupervisorMessage =
  | {
      type: 'memory';
      pid: number;
      buffer: SharedArrayBuffer;
      workerId: number;
    }
  | {
      type: 'error';
      pid: number;
      workerId: number;
      error: string;
    };
