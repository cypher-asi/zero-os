/**
 * Zero OS Worker - JS Object Heap
 *
 * Manages a heap of JavaScript objects for wasm-bindgen compatibility.
 * wasm-bindgen stores JS objects in a heap and passes indices to WASM.
 */

/**
 * WasmBindgenHeap manages JavaScript object references for WASM code.
 *
 * Objects are stored at indices in a heap array. Pre-populated values
 * (undefined, null, true, false) occupy the first slots after the
 * reserved area.
 */
export class WasmBindgenHeap {
  private heap: unknown[];
  private heapNext: number;

  // Reserved indices for pre-populated values
  private static readonly RESERVED_SIZE = 128;
  private static readonly UNDEFINED_IDX = 128;
  private static readonly NULL_IDX = 129;
  private static readonly TRUE_IDX = 130;
  private static readonly FALSE_IDX = 131;
  private static readonly FIRST_FREE_IDX = 132;

  constructor() {
    this.heap = new Array(WasmBindgenHeap.RESERVED_SIZE).fill(undefined);
    // Pre-populate with common values at known indices
    this.heap.push(undefined, null, true, false);
    this.heapNext = this.heap.length;
  }

  /**
   * Add an object to the heap and return its index
   */
  addObject(obj: unknown): number {
    if (this.heapNext === this.heap.length) {
      this.heap.push(this.heap.length + 1);
    }
    const idx = this.heapNext;
    this.heapNext = this.heap[idx] as number;
    this.heap[idx] = obj;
    return idx;
  }

  /**
   * Get an object from the heap by index
   */
  getObject(idx: number): unknown {
    return this.heap[idx];
  }

  /**
   * Drop an object reference, returning its slot to the free list
   */
  dropObject(idx: number): void {
    // Don't drop pre-populated values
    if (idx < WasmBindgenHeap.FIRST_FREE_IDX) return;
    this.heap[idx] = this.heapNext;
    this.heapNext = idx;
  }

  /**
   * Take an object from the heap (get and drop in one operation)
   */
  takeObject(idx: number): unknown {
    const ret = this.getObject(idx);
    this.dropObject(idx);
    return ret;
  }
}
