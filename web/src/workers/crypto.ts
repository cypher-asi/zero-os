/**
 * Zero OS Worker - Crypto Utilities
 *
 * SharedArrayBuffer-safe crypto operations for getrandom support.
 */

declare const self: DedicatedWorkerGlobalScope;

/**
 * Fill a buffer with cryptographically secure random values.
 * Handles SharedArrayBuffer by copying through a non-shared buffer,
 * since crypto.getRandomValues doesn't support SharedArrayBuffer directly.
 *
 * @param memory - WASM memory instance
 * @param ptr - Pointer into WASM memory
 * @param len - Number of random bytes to generate
 * @param workerId - Worker ID for logging
 * @param pid - Process ID for logging
 */
export function getRandomValues(
  memory: WebAssembly.Memory,
  ptr: number,
  len: number,
  workerId: number,
  pid: number
): void {
  console.log(
    `[worker:${workerId}:${pid}] __wbindgen_get_random_values called: ptr=${ptr}, len=${len}`
  );

  if (!memory) {
    console.error(
      `[worker:${workerId}:${pid}] CRYPTO ERROR: Memory not initialized`
    );
    throw new Error('Memory not initialized for getRandomValues');
  }

  const buf = new Uint8Array(memory.buffer, ptr, len);

  // Check if backed by SharedArrayBuffer - getRandomValues doesn't support it
  if (memory.buffer instanceof SharedArrayBuffer) {
    // Create a non-shared copy, fill it, then copy back
    const copy = new Uint8Array(len);
    self.crypto.getRandomValues(copy);
    buf.set(copy);

    // Verify we got non-zero random bytes
    const allZeros = copy.every((b) => b === 0);
    if (allZeros && len > 0) {
      console.error(
        `[worker:${workerId}:${pid}] CRYPTO WARNING: getRandomValues returned all zeros!`
      );
    }
  } else {
    self.crypto.getRandomValues(buf);
  }

  // Log first few bytes for debugging (only for small buffers to avoid spam)
  if (len <= 32) {
    const preview = Array.from(buf.slice(0, Math.min(8, len)))
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
    console.log(`[worker:${workerId}:${pid}] Random bytes preview: ${preview}...`);
  }
}

/**
 * Fill a Uint8Array object with random values.
 * Used by proxy handler for crypto.getRandomValues calls on JS objects.
 *
 * @param array - The Uint8Array to fill
 * @param workerId - Worker ID for logging
 * @param pid - Process ID for logging
 */
export function fillRandomArray(
  array: Uint8Array,
  workerId: number,
  pid: number
): void {
  // Check if backed by SharedArrayBuffer
  if (array.buffer instanceof SharedArrayBuffer) {
    const copy = new Uint8Array(array.length);
    self.crypto.getRandomValues(copy);
    array.set(copy);

    const preview = Array.from(copy.slice(0, 8))
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
    console.log(
      `[worker:${workerId}:${pid}] PROXY random bytes (SharedAB): ${preview}...`
    );
  } else {
    self.crypto.getRandomValues(array);

    const preview = Array.from(array.slice(0, 8))
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
    console.log(`[worker:${workerId}:${pid}] PROXY random bytes: ${preview}...`);
  }
}
