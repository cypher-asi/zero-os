/**
 * Tests for Worker Types
 *
 * Tests for type utilities, constants, and the createWorkerState function.
 */

import { describe, it, expect } from 'vitest';
import {
  STATUS_IDLE,
  STATUS_PENDING,
  STATUS_READY,
  MAILBOX_OFFSETS,
  MAILBOX_DATA_BYTE_OFFSET,
  MAILBOX_MAX_DATA_LEN,
  createWorkerState,
} from '../types';

describe('Mailbox Status Constants', () => {
  it('should have STATUS_IDLE as 0', () => {
    expect(STATUS_IDLE).toBe(0);
  });

  it('should have STATUS_PENDING as 1', () => {
    expect(STATUS_PENDING).toBe(1);
  });

  it('should have STATUS_READY as 2', () => {
    expect(STATUS_READY).toBe(2);
  });
});

describe('Mailbox Offsets', () => {
  it('should have STATUS at offset 0', () => {
    expect(MAILBOX_OFFSETS.STATUS).toBe(0);
  });

  it('should have SYSCALL_NUM at offset 1', () => {
    expect(MAILBOX_OFFSETS.SYSCALL_NUM).toBe(1);
  });

  it('should have ARG offsets at 2, 3, 4', () => {
    expect(MAILBOX_OFFSETS.ARG0).toBe(2);
    expect(MAILBOX_OFFSETS.ARG1).toBe(3);
    expect(MAILBOX_OFFSETS.ARG2).toBe(4);
  });

  it('should have RESULT at offset 5', () => {
    expect(MAILBOX_OFFSETS.RESULT).toBe(5);
  });

  it('should have DATA_LEN at offset 6', () => {
    expect(MAILBOX_OFFSETS.DATA_LEN).toBe(6);
  });

  it('should have DATA at offset 7 (byte offset 28)', () => {
    expect(MAILBOX_OFFSETS.DATA).toBe(7);
    // DATA at i32 offset 7 means byte offset 7 * 4 = 28
    expect(MAILBOX_OFFSETS.DATA * 4).toBe(MAILBOX_DATA_BYTE_OFFSET);
  });

  it('should have PID at offset 14', () => {
    expect(MAILBOX_OFFSETS.PID).toBe(14);
  });
});

describe('Mailbox Data Constants', () => {
  it('should have MAILBOX_DATA_BYTE_OFFSET as 28', () => {
    expect(MAILBOX_DATA_BYTE_OFFSET).toBe(28);
  });

  it('should have MAILBOX_MAX_DATA_LEN as 16356', () => {
    expect(MAILBOX_MAX_DATA_LEN).toBe(16356);
  });
});

describe('createWorkerState', () => {
  it('should create a fresh uninitialized state', () => {
    const state = createWorkerState(42);

    expect(state.initialized).toBe(false);
    expect(state.pid).toBe(0);
    expect(state.workerId).toBe(42);
    expect(state.wasmMemory).toBe(null);
    expect(state.mailboxView).toBe(null);
    expect(state.mailboxBytes).toBe(null);
  });

  it('should use provided workerId', () => {
    const state1 = createWorkerState(1);
    const state2 = createWorkerState(999);

    expect(state1.workerId).toBe(1);
    expect(state2.workerId).toBe(999);
  });

  it('should create independent state objects', () => {
    const state1 = createWorkerState(1);
    const state2 = createWorkerState(2);

    state1.initialized = true;
    state1.pid = 100;

    expect(state2.initialized).toBe(false);
    expect(state2.pid).toBe(0);
  });
});
