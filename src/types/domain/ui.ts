/**
 * UI domain type definitions
 */

// ===== Component base types =====

export type ThemeMode = 'light' | 'dark' | 'auto'
export type Placement = 'top' | 'bottom' | 'left' | 'right'

export interface SelectOption {
  label: string
  value: string | number
  disabled?: boolean
}
