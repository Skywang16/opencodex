/**
 * Shortcut system utility functions
 */

import { KEY_NORMALIZATION_MAP, MODIFIER_KEYS } from './constants'
import type { ShortcutBinding } from '@/types'

/**
 * Normalize key name
 */
export const normalizeKey = (key: string): string => {
  return KEY_NORMALIZATION_MAP[key] || key.toLowerCase()
}

/**
 * Get event modifier keys
 */
export const getEventModifiers = (event: KeyboardEvent): string[] => {
  const modifiers: string[] = []

  if (event.ctrlKey) modifiers.push(MODIFIER_KEYS.CTRL)
  if (event.altKey) modifiers.push(MODIFIER_KEYS.ALT)
  if (event.shiftKey) modifiers.push(MODIFIER_KEYS.SHIFT)
  if (event.metaKey || event.ctrlKey) {
    // macOS uses cmd, other platforms use ctrl
    if (navigator.platform.includes('Mac')) {
      if (event.metaKey) modifiers.push(MODIFIER_KEYS.CMD)
    } else {
      if (event.ctrlKey) modifiers.push(MODIFIER_KEYS.CMD)
    }
  }

  return modifiers.sort()
}

/**
 * Normalize modifier key array
 */
export const normalizeModifiers = (modifiers: string[]): string[] => {
  return modifiers.map(m => m.toLowerCase()).sort()
}

/**
 * Compare if modifier keys are equal
 */
export const areModifiersEqual = (mods1: string[], mods2: string[]): boolean => {
  if (mods1.length !== mods2.length) return false

  for (let i = 0; i < mods1.length; i++) {
    if (mods1[i] !== mods2[i]) return false
  }

  return true
}

/**
 * Format key combination as string
 */
export const formatKeyCombo = (event: KeyboardEvent): string => {
  const modifiers = getEventModifiers(event)
  const key = normalizeKey(event.key)

  if (modifiers.length > 0) {
    return `${modifiers.join('+')}+${key}`
  }
  return key
}

/**
 * Check if key event matches shortcut
 */
export const isShortcutMatch = (event: KeyboardEvent, shortcut: ShortcutBinding): boolean => {
  // Check main key
  const normalizedKey = normalizeKey(event.key)
  const shortcutKey = normalizeKey(shortcut.key)

  if (normalizedKey !== shortcutKey) {
    return false
  }

  // Check modifier keys
  const eventModifiers = getEventModifiers(event)
  const shortcutModifiers = normalizeModifiers(shortcut.modifiers)

  return areModifiersEqual(eventModifiers, shortcutModifiers)
}

/**
 * Extract action name
 */
export const extractActionName = (action: string): string => {
  return action
}

/**
 * Check if it's a platform-specific shortcut
 */
export const isPlatformShortcut = (keyCombo: string): boolean => {
  const isMac = navigator.platform.includes('Mac')

  // Common platform shortcuts
  const macShortcuts = ['cmd+c', 'cmd+v', 'cmd+x', 'cmd+z', 'cmd+a', 'cmd+s']
  const winShortcuts = ['ctrl+c', 'ctrl+v', 'ctrl+x', 'ctrl+z', 'ctrl+a', 'ctrl+s']

  if (isMac) {
    return macShortcuts.includes(keyCombo.toLowerCase())
  } else {
    return winShortcuts.includes(keyCombo.toLowerCase())
  }
}

/**
 * Generate debug information
 */
export const generateDebugInfo = (event: KeyboardEvent, shortcut?: ShortcutBinding) => {
  return {
    timestamp: new Date().toISOString(),
    keyInfo: {
      key: event.key,
      code: event.code,
      normalizedKey: normalizeKey(event.key),
      modifiers: getEventModifiers(event),
      keyCombo: formatKeyCombo(event),
    },
    shortcutInfo: shortcut
      ? {
          key: shortcut.key,
          modifiers: shortcut.modifiers,
          action: extractActionName(shortcut.action),
          normalizedKey: normalizeKey(shortcut.key),
          normalizedModifiers: normalizeModifiers(shortcut.modifiers),
        }
      : null,
    browserInfo: {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      isMac: navigator.platform.includes('Mac'),
    },
  }
}
