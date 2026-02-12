/**
 * Shortcut system constant definitions
 */

/**
 * Supported modifier keys
 */
export const MODIFIER_KEYS = {
  CMD: 'cmd',
  CTRL: 'ctrl',
  ALT: 'alt',
  SHIFT: 'shift',
  META: 'meta',
} as const

/**
 * Key normalization mapping
 */
export const KEY_NORMALIZATION_MAP: Record<string, string> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ' ': 'space',
  Enter: 'return',
  Escape: 'esc',
  Backspace: 'backspace',
  Delete: 'delete',
  Tab: 'tab',
}

/**
 * Shortcut action definitions
 * Action key -> action name mapping (for internal use)
 * Display names should be handled by i18n system
 */
export const SHORTCUT_ACTIONS = {
  // Global actions
  copy_to_clipboard: 'copy_to_clipboard',
  paste_from_clipboard: 'paste_from_clipboard',
  command_palette: 'command_palette',
  terminal_search: 'terminal_search',
  open_settings: 'open_settings',

  // Terminal functions
  new_terminal: 'new_terminal',
  clear_terminal: 'clear_terminal',
  accept_completion: 'accept_completion',
  toggle_terminal_panel: 'toggle_terminal_panel',

  // UI
  toggle_window_pin: 'toggle_window_pin',
} as const
