/**
 * Identity utility functions shared across hooks and services.
 */

/**
 * Convert a user ID (string, number, or bigint) to BigInt.
 * Handles hex strings (0x prefix), decimal strings, and numeric types.
 *
 * @param userId - The user ID to convert (can be string, number, bigint, null, or undefined)
 * @returns The user ID as BigInt, or null if invalid/empty
 */
export function userIdToBigInt(userId: string | number | bigint | null | undefined): bigint | null {
  if (userId === null || userId === undefined) return null;
  if (typeof userId === 'bigint') return userId;
  if (typeof userId === 'string') {
    if (!userId) return null;
    // Handle hex strings (e.g., "0x123abc")
    if (userId.startsWith('0x')) {
      try {
        return BigInt(userId);
      } catch {
        return null;
      }
    }
    // Handle decimal strings
    try {
      return BigInt(userId);
    } catch {
      return null;
    }
  }
  // Handle numbers
  try {
    return BigInt(userId);
  } catch {
    return null;
  }
}

// ============================================================================
// Byte/Hex Conversion Utilities
// ============================================================================

/**
 * Convert a byte array (or string) to a hex string.
 *
 * @param bytes - Array of numbers (0-255) or a string (returned as-is)
 * @returns Hexadecimal string representation (e.g., "0a1b2c")
 *
 * @example
 * bytesToHex([10, 27, 44]) // => "0a1b2c"
 * bytesToHex("already-hex") // => "already-hex"
 */
export function bytesToHex(bytes: number[] | string): string {
  if (typeof bytes === 'string') return bytes;
  return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Convert a u128 value (number or string) to a 32-character padded hex string.
 *
 * JavaScript numbers cannot safely represent u128 values (max safe integer is 2^53-1),
 * so this function handles both number and string inputs. The output is always
 * a 32-character hex string with "0x" prefix (e.g., "0x00000000000000000000000000000001").
 *
 * @param value - A u128 value as number, string, or bigint
 * @returns A "0x"-prefixed, 32-character padded hex string
 *
 * @example
 * u128ToHex(1) // => "0x00000000000000000000000000000001"
 * u128ToHex("42") // => "0x0000000000000000000000000000002a"
 * u128ToHex(BigInt("0x123abc")) // => "0x00000000000000000000000000123abc"
 */
export function u128ToHex(value: number | string | bigint): string {
  if (typeof value === 'number') {
    return '0x' + value.toString(16).padStart(32, '0');
  }
  if (typeof value === 'bigint') {
    return '0x' + value.toString(16).padStart(32, '0');
  }
  // String: try to parse as BigInt for proper conversion
  try {
    const bigVal = BigInt(value);
    return '0x' + bigVal.toString(16).padStart(32, '0');
  } catch {
    // If parsing fails, just return as string (likely already formatted)
    return value.toString();
  }
}
