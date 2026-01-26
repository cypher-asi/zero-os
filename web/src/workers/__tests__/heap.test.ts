/**
 * Tests for WasmBindgenHeap
 *
 * Pure unit tests for the JavaScript object heap used by wasm-bindgen shims.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { WasmBindgenHeap } from '../heap';

describe('WasmBindgenHeap', () => {
  let heap: WasmBindgenHeap;

  beforeEach(() => {
    heap = new WasmBindgenHeap();
  });

  describe('pre-populated values', () => {
    it('should have undefined at index 128', () => {
      expect(heap.getObject(128)).toBe(undefined);
    });

    it('should have null at index 129', () => {
      expect(heap.getObject(129)).toBe(null);
    });

    it('should have true at index 130', () => {
      expect(heap.getObject(130)).toBe(true);
    });

    it('should have false at index 131', () => {
      expect(heap.getObject(131)).toBe(false);
    });
  });

  describe('addObject', () => {
    it('should add an object and return its index', () => {
      const obj = { test: 'value' };
      const idx = heap.addObject(obj);
      expect(idx).toBeGreaterThanOrEqual(132); // First free index after pre-populated
      expect(heap.getObject(idx)).toBe(obj);
    });

    it('should add multiple objects with sequential indices', () => {
      const obj1 = { id: 1 };
      const obj2 = { id: 2 };
      const obj3 = { id: 3 };

      const idx1 = heap.addObject(obj1);
      const idx2 = heap.addObject(obj2);
      const idx3 = heap.addObject(obj3);

      expect(heap.getObject(idx1)).toBe(obj1);
      expect(heap.getObject(idx2)).toBe(obj2);
      expect(heap.getObject(idx3)).toBe(obj3);
    });

    it('should handle adding primitive values', () => {
      const numIdx = heap.addObject(42);
      const strIdx = heap.addObject('hello');

      expect(heap.getObject(numIdx)).toBe(42);
      expect(heap.getObject(strIdx)).toBe('hello');
    });

    it('should handle adding functions', () => {
      const fn = () => 'test';
      const idx = heap.addObject(fn);
      expect(heap.getObject(idx)).toBe(fn);
    });
  });

  describe('dropObject', () => {
    it('should drop an object and allow its slot to be reused', () => {
      const obj1 = { id: 1 };
      const idx1 = heap.addObject(obj1);
      heap.dropObject(idx1);

      // The next addObject should reuse the dropped slot
      const obj2 = { id: 2 };
      const idx2 = heap.addObject(obj2);
      expect(idx2).toBe(idx1);
      expect(heap.getObject(idx2)).toBe(obj2);
    });

    it('should not drop pre-populated values (index < 132)', () => {
      // Try to drop the pre-populated undefined
      heap.dropObject(128);
      expect(heap.getObject(128)).toBe(undefined);

      // Try to drop null
      heap.dropObject(129);
      expect(heap.getObject(129)).toBe(null);

      // Try to drop true
      heap.dropObject(130);
      expect(heap.getObject(130)).toBe(true);

      // Try to drop false
      heap.dropObject(131);
      expect(heap.getObject(131)).toBe(false);
    });

    it('should handle dropping the same index twice gracefully', () => {
      const obj = { test: 'value' };
      const idx = heap.addObject(obj);

      // Drop twice - should not break
      heap.dropObject(idx);
      heap.dropObject(idx);

      // Should still be able to add new objects
      const newObj = { new: 'object' };
      const newIdx = heap.addObject(newObj);
      expect(heap.getObject(newIdx)).toBe(newObj);
    });
  });

  describe('takeObject', () => {
    it('should get and drop an object in one operation', () => {
      const obj = { test: 'value' };
      const idx = heap.addObject(obj);

      const taken = heap.takeObject(idx);
      expect(taken).toBe(obj);

      // Slot should be reused
      const newObj = { new: 'object' };
      const newIdx = heap.addObject(newObj);
      expect(newIdx).toBe(idx);
    });

    it('should return pre-populated values without dropping', () => {
      const takenUndefined = heap.takeObject(128);
      const takenNull = heap.takeObject(129);
      const takenTrue = heap.takeObject(130);
      const takenFalse = heap.takeObject(131);

      expect(takenUndefined).toBe(undefined);
      expect(takenNull).toBe(null);
      expect(takenTrue).toBe(true);
      expect(takenFalse).toBe(false);

      // Pre-populated values should still be there
      expect(heap.getObject(128)).toBe(undefined);
      expect(heap.getObject(129)).toBe(null);
      expect(heap.getObject(130)).toBe(true);
      expect(heap.getObject(131)).toBe(false);
    });
  });

  describe('free list reuse', () => {
    it('should maintain LIFO order for dropped slots', () => {
      const obj1 = { id: 1 };
      const obj2 = { id: 2 };
      const obj3 = { id: 3 };

      const idx1 = heap.addObject(obj1);
      const idx2 = heap.addObject(obj2);
      const idx3 = heap.addObject(obj3);

      // Drop in order: 1, 2, 3
      heap.dropObject(idx1);
      heap.dropObject(idx2);
      heap.dropObject(idx3);

      // Add back - should reuse in LIFO order: 3, 2, 1
      const newIdx1 = heap.addObject({ new: 1 });
      const newIdx2 = heap.addObject({ new: 2 });
      const newIdx3 = heap.addObject({ new: 3 });

      expect(newIdx1).toBe(idx3);
      expect(newIdx2).toBe(idx2);
      expect(newIdx3).toBe(idx1);
    });
  });

  describe('many objects', () => {
    it('should handle adding many objects', () => {
      const objects: object[] = [];
      const indices: number[] = [];

      for (let i = 0; i < 100; i++) {
        const obj = { index: i };
        objects.push(obj);
        indices.push(heap.addObject(obj));
      }

      // Verify all objects are stored correctly
      for (let i = 0; i < 100; i++) {
        expect(heap.getObject(indices[i])).toBe(objects[i]);
      }
    });
  });
});
