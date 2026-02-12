import type { Theme } from '@/types'

/**
 * XTerm.js theme interface
 * Based on XTerm.js official documentation
 */
export interface XTermTheme {
  /** Foreground color */
  foreground?: string
  /** Background color */
  background?: string
  /** Cursor color */
  cursor?: string
  /** Cursor accent color */
  cursorAccent?: string
  /** Selection background color */
  selectionBackground?: string
  /** Selection foreground color */
  selectionForeground?: string
  /** Inactive selection background color */
  selectionInactiveBackground?: string

  // ANSI colors (0-7)
  /** ANSI black */
  black?: string
  /** ANSI red */
  red?: string
  /** ANSI green */
  green?: string
  /** ANSI yellow */
  yellow?: string
  /** ANSI blue */
  blue?: string
  /** ANSI magenta */
  magenta?: string
  /** ANSI cyan */
  cyan?: string
  /** ANSI white */
  white?: string

  // Bright ANSI colors (8-15)
  /** Bright black */
  brightBlack?: string
  /** Bright red */
  brightRed?: string
  /** Bright green */
  brightGreen?: string
  /** Bright yellow */
  brightYellow?: string
  /** Bright blue */
  brightBlue?: string
  /** Bright magenta */
  brightMagenta?: string
  /** Bright cyan */
  brightCyan?: string
  /** Bright white */
  brightWhite?: string
}

/**
 * Convert project theme data to XTerm.js theme format
 *
 * @param theme Project theme data
 * @returns XTerm.js theme object
 */
export const convertThemeToXTerm = (theme: Theme): XTermTheme => {
  return {
    foreground: theme.ui.text_200,

    background: 'transparent',
    cursor: theme.ui.text_100,
    selectionBackground: theme.ui.selection,

    black: theme.ansi.black,
    red: theme.ansi.red,
    green: theme.ansi.green,
    yellow: theme.ansi.yellow,
    blue: theme.ansi.blue,
    magenta: theme.ansi.magenta,
    cyan: theme.ansi.cyan,
    white: theme.ansi.white,

    brightBlack: theme.bright.black,
    brightRed: theme.bright.red,
    brightGreen: theme.bright.green,
    brightYellow: theme.bright.yellow,
    brightBlue: theme.bright.blue,
    brightMagenta: theme.bright.magenta,
    brightCyan: theme.bright.cyan,
    brightWhite: theme.bright.white,
  }
}

/**
 * Create default XTerm.js theme
 * Used when theme data cannot be retrieved
 *
 * @returns Default XTerm.js theme object
 */
export const createDefaultXTermTheme = (): XTermTheme => {
  return {
    foreground: '#f0f0f0',
    background: 'transparent',
    cursor: '#ffffff',
    selectionBackground: '#3391ff',

    black: '#000000',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5',

    brightBlack: '#666666',
    brightRed: '#f14c4c',
    brightGreen: '#23d18b',
    brightYellow: '#f5f543',
    brightBlue: '#3b8eea',
    brightMagenta: '#d670d6',
    brightCyan: '#29b8db',
    brightWhite: '#ffffff',
  }
}
