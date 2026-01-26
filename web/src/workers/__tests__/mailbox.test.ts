/**
 * Tests for Mailbox Operations
 *
 * Integration tests for SharedArrayBuffer + Atomics based syscall mailbox.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  refreshViews,
  zos_send_bytes,
  zos_recv_bytes,
  zos_get_pid,
  zos_yield,
} from '../mailbox';
import {
  MAILBOX_OFFSETS,
  MAILBOX_DATA_BYTE_OFFSET,
  MAILBOX_MAX_DATA_LEN,
  STATUS_IDLE,
} from '../types';
import {
  createTestState,
  createMockPostMemoryUpdate,
  writeBytes,
  readBytes,
} from './helpers';

describe('zos_send_bytes', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should copy data from WASM memory to mailbox buffer', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Write some data to WASM memory
    const testData = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
    const srcPtr = 4096; // Arbitrary location in WASM memory
    writeBytes(state.wasmMemory!, srcPtr, testData);

    // Send the bytes
    const sent = zos_send_bytes(state, postMemoryUpdate.fn, srcPtr, testData.length);

    // Verify bytes sent
    expect(sent).toBe(testData.length);

    // Verify data was copied to mailbox
    const mailboxData = readBytes(
      state.wasmMemory!,
      MAILBOX_DATA_BYTE_OFFSET,
      testData.length
    );
    expect(Array.from(mailboxData)).toEqual(Array.from(testData));

    // Verify DATA_LEN was set
    const dataLen = Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN);
    expect(dataLen).toBe(testData.length);
  });

  it('should clamp data length to MAILBOX_MAX_DATA_LEN', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Try to send more than max
    const oversizeLen = MAILBOX_MAX_DATA_LEN + 1000;
    const sent = zos_send_bytes(state, postMemoryUpdate.fn, 4096, oversizeLen);

    expect(sent).toBe(MAILBOX_MAX_DATA_LEN);
  });

  it('should handle zero-length data', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    const sent = zos_send_bytes(state, postMemoryUpdate.fn, 4096, 0);

    expect(sent).toBe(0);
    const dataLen = Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN);
    expect(dataLen).toBe(0);
  });

  it('should work with SharedArrayBuffer memory', () => {
    const state = createTestState();
    expect(state.wasmMemory!.buffer).toBeInstanceOf(SharedArrayBuffer);

    const postMemoryUpdate = createMockPostMemoryUpdate();
    const testData = new Uint8Array([10, 20, 30, 40]);
    writeBytes(state.wasmMemory!, 4096, testData);

    const sent = zos_send_bytes(state, postMemoryUpdate.fn, 4096, testData.length);

    expect(sent).toBe(4);
    const mailboxData = readBytes(
      state.wasmMemory!,
      MAILBOX_DATA_BYTE_OFFSET,
      testData.length
    );
    expect(Array.from(mailboxData)).toEqual([10, 20, 30, 40]);
  });
});

describe('zos_recv_bytes', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should copy data from mailbox buffer to WASM memory', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Put data in mailbox buffer
    const testData = new Uint8Array([100, 101, 102, 103, 104]);
    writeBytes(state.wasmMemory!, MAILBOX_DATA_BYTE_OFFSET, testData);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, testData.length);

    // Receive into WASM memory
    const dstPtr = 8192;
    const received = zos_recv_bytes(
      state,
      postMemoryUpdate.fn,
      dstPtr,
      testData.length
    );

    expect(received).toBe(testData.length);
    const result = readBytes(state.wasmMemory!, dstPtr, testData.length);
    expect(Array.from(result)).toEqual([100, 101, 102, 103, 104]);
  });

  it('should limit received bytes to maxLen', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Put 10 bytes in mailbox
    const testData = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    writeBytes(state.wasmMemory!, MAILBOX_DATA_BYTE_OFFSET, testData);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, testData.length);

    // Only receive 5 bytes
    const dstPtr = 8192;
    const received = zos_recv_bytes(state, postMemoryUpdate.fn, dstPtr, 5);

    expect(received).toBe(5);
    const result = readBytes(state.wasmMemory!, dstPtr, 5);
    expect(Array.from(result)).toEqual([1, 2, 3, 4, 5]);
  });

  it('should handle zero available data', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, 0);

    const dstPtr = 8192;
    const received = zos_recv_bytes(state, postMemoryUpdate.fn, dstPtr, 100);

    expect(received).toBe(0);
  });

  it('should return min of dataLen and maxLen', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Less data available than requested
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, 3);
    const testData = new Uint8Array([7, 8, 9]);
    writeBytes(state.wasmMemory!, MAILBOX_DATA_BYTE_OFFSET, testData);

    const received = zos_recv_bytes(state, postMemoryUpdate.fn, 8192, 100);
    expect(received).toBe(3);
  });
});

describe('zos_get_pid', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should read PID from mailbox', () => {
    const state = createTestState(123);
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Store PID in mailbox
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.PID, 123);

    const pid = zos_get_pid(state, postMemoryUpdate.fn);
    expect(pid).toBe(123);
  });

  it('should read different PID values', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.PID, 456);
    expect(zos_get_pid(state, postMemoryUpdate.fn)).toBe(456);

    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.PID, 1);
    expect(zos_get_pid(state, postMemoryUpdate.fn)).toBe(1);
  });
});

describe('zos_yield', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should not throw and should return quickly', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Set status to something other than IDLE so wait returns immediately
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.STATUS, STATUS_IDLE);

    const start = Date.now();
    zos_yield(state, postMemoryUpdate.fn);
    const elapsed = Date.now() - start;

    // Should return within reasonable time (Atomics.wait has 1ms timeout)
    expect(elapsed).toBeLessThan(100);
  });
});

describe('refreshViews', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should not update views when buffer has not changed', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    const originalMailboxView = state.mailboxView;
    const originalMailboxBytes = state.mailboxBytes;

    refreshViews(state, postMemoryUpdate.fn);

    expect(state.mailboxView).toBe(originalMailboxView);
    expect(state.mailboxBytes).toBe(originalMailboxBytes);
    expect(postMemoryUpdate.calls).toBe(0);
  });

  it('should update views and call postMemoryUpdate when buffer changes', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Store some state in mailbox
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.STATUS, 1);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.RESULT, 42);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, 0);

    const originalBuffer = state.wasmMemory!.buffer;

    // Simulate memory growth by creating a new memory and setting a different buffer reference
    // This is tricky to test without actually growing, so we'll simulate by manually
    // invalidating the view's buffer reference
    state.wasmMemory!.grow(1);

    // The buffer should now be different
    if (state.wasmMemory!.buffer !== originalBuffer) {
      refreshViews(state, postMemoryUpdate.fn);

      // Views should be updated
      expect(state.mailboxView!.buffer).toBe(state.wasmMemory!.buffer);
      expect(state.mailboxBytes!.buffer).toBe(state.wasmMemory!.buffer);

      // postMemoryUpdate should have been called
      expect(postMemoryUpdate.calls).toBe(1);
    }
  });

  it('should preserve mailbox state across view refresh', () => {
    const state = createTestState(99);
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Store state before growth
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.STATUS, 1);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.RESULT, 777);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, 0);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.PID, 99);

    const originalBuffer = state.wasmMemory!.buffer;
    state.wasmMemory!.grow(1);

    if (state.wasmMemory!.buffer !== originalBuffer) {
      refreshViews(state, postMemoryUpdate.fn);

      // Verify state was preserved
      expect(Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.STATUS)).toBe(1);
      expect(Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.RESULT)).toBe(777);
      expect(Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.PID)).toBe(99);
    }
  });

  it('should preserve data buffer contents across view refresh', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Write data to mailbox
    const testData = new Uint8Array([11, 22, 33, 44, 55]);
    writeBytes(state.wasmMemory!, MAILBOX_DATA_BYTE_OFFSET, testData);
    Atomics.store(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN, testData.length);

    const originalBuffer = state.wasmMemory!.buffer;
    state.wasmMemory!.grow(1);

    if (state.wasmMemory!.buffer !== originalBuffer) {
      refreshViews(state, postMemoryUpdate.fn);

      // Verify data was preserved
      const dataLen = Atomics.load(state.mailboxView!, MAILBOX_OFFSETS.DATA_LEN);
      expect(dataLen).toBe(testData.length);

      const preserved = readBytes(
        state.wasmMemory!,
        MAILBOX_DATA_BYTE_OFFSET,
        testData.length
      );
      expect(Array.from(preserved)).toEqual([11, 22, 33, 44, 55]);
    }
  });
});

describe('mailbox integration', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should round-trip data through send and recv', () => {
    const state = createTestState();
    const postMemoryUpdate = createMockPostMemoryUpdate();

    // Write data to WASM memory
    const originalData = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    const srcPtr = 4096;
    writeBytes(state.wasmMemory!, srcPtr, originalData);

    // Send to mailbox
    zos_send_bytes(state, postMemoryUpdate.fn, srcPtr, originalData.length);

    // Receive to different location
    const dstPtr = 8192;
    const received = zos_recv_bytes(
      state,
      postMemoryUpdate.fn,
      dstPtr,
      originalData.length
    );

    // Verify round-trip
    expect(received).toBe(originalData.length);
    const result = readBytes(state.wasmMemory!, dstPtr, originalData.length);
    expect(Array.from(result)).toEqual(Array.from(originalData));
  });
});
