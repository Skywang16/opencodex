/**
 * Extract the last part of a full path as display name
 * @param path Full path
 * @returns Last part of path, returns '~' if root directory or empty
 */
export const getPathBasename = (path: string): string => {
  if (!path || path === '~') return '~'

  const parts = path.replace(/[/\\]+$/, '').split(/[/\\]/)
  const basename = parts[parts.length - 1]

  return basename || '~'
}
