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
