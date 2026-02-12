/**
 * FNV-1a 32-bit hash algorithm
 * Used to generate stable string hash values, suitable as cache keys or deduplication identifiers
 */
export const fnv1aHash = (str: string): string => {
  let hash = 2166136261 // FNV offset basis
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i)
    hash = Math.imul(hash, 16777619) // FNV prime
  }
  return (hash >>> 0).toString(16)
}
