import type { TerminalConfig } from '@/types'

// Terminal configuration - optimized for performance and rendering
export const TERMINAL_CONFIG: TerminalConfig = {
  // BaseConfig properties
  version: '1.0.0',
  lastModified: new Date().toISOString(),
  enabled: true,

  fontFamily:
    '"JetBrainsMono Nerd Font", "FiraCode Nerd Font", "Fira Code", "JetBrains Mono", Menlo, Monaco, "SF Mono", "Microsoft YaHei UI", "PingFang SC", "Hiragino Sans GB", "Source Han Sans CN", "WenQuanYi Micro Hei", "Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", "Courier New", monospace',
  fontSize: 14,
  allowProposedApi: true,
  allowTransparency: true,
  cursorBlink: true,
  theme: {
    background: '#1e1e1e',
    foreground: '#f0f0f0',
  },
  scrollback: 1000, // Reduce scrollback buffer to improve performance

  // Required configuration objects
  shell: {
    default: '/bin/zsh',
    args: [],
    workingDirectory: '~',
  },
  cursor: {
    style: 'block',
    blink: true,
    color: '#f0f0f0',
    thickness: 1,
  },
  behavior: {
    closeOnExit: false,
    confirmOnExit: true,
    scrollOnOutput: true,
    copyOnSelect: false,
  },

  convertEol: false,
  cursorStyle: 'block',
  drawBoldTextInBrightColors: true,
  fontWeight: 400,
  fontWeightBold: 700,
  letterSpacing: 0,
  lineHeight: 1.2,

  // Chinese and internationalization optimizations
  macOptionIsMeta: false, // On Mac, Option key is not used as Meta key to avoid Chinese input issues
  minimumContrastRatio: 1, // Use original colors, avoid forced brightening to white in light themes
  rightClickSelectsWord: false, // Avoid right-click selection interfering with Chinese words
  wordSeparator: ' ()[]{}\'",;', // Word separator optimized for Chinese

  // Scroll and buffer optimizations - performance tuning for Canvas renderer
  scrollSensitivity: 1, // Reduce scroll sensitivity to decrease event frequency
  fastScrollSensitivity: 5, // Skip more lines during fast scrolling to reduce render count
  smoothScrollDuration: 0, // Disable smooth scrolling to reduce rendering overhead

  // Other performance optimizations
  tabStopWidth: 4, // Reduce tab width to better match modern programming habits
  screenReaderMode: false, // Disable screen reader mode to improve performance
  windowsMode: false, // Disable Windows mode to reduce compatibility overhead
}

// Terminal event constants
export const TERMINAL_EVENTS = {
  EXIT: 'terminal_exit',
} as const
