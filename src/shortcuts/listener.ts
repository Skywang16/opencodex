/**
 * Shortcut listener composable
 *
 * Listens to global keyboard events and executes corresponding shortcut actions
 */

import { shortcutsApi } from '@/api/shortcuts'
import { useTerminalStore } from '@/stores/Terminal'
import type { ShortcutBinding, ShortcutsConfig } from '@/types'
import { onMounted, onUnmounted, ref } from 'vue'
import { shortcutActionsService } from './actions'
import { extractActionName, formatKeyCombo, isShortcutMatch } from './utils'

export const useShortcutListener = () => {
  const isListening = ref(false)
  const config = ref<ShortcutsConfig | null>(null)
  let keydownHandler: ((event: KeyboardEvent) => void) | null = null
  const terminalStore = useTerminalStore()

  const initializeListener = async () => {
    config.value = await shortcutsApi.getConfig()

    keydownHandler = (event: KeyboardEvent) => {
      handleKeyDown(event)
    }

    document.addEventListener('keydown', keydownHandler, true)
    isListening.value = true
  }

  const handleKeyDown = async (event: KeyboardEvent) => {
    if (!config.value) return

    const keyCombo = formatKeyCombo(event)
    const matchedShortcut = findMatchingShortcut(event, config.value)

    if (matchedShortcut) {
      const actionName = extractActionName(matchedShortcut.action)

      // Don't prevent default for copy/paste, prevent all others
      // Must call preventDefault in sync phase, otherwise system default may have triggered
      if (actionName !== 'copy_to_clipboard' && actionName !== 'paste_from_clipboard') {
        event.preventDefault()
        event.stopPropagation()
      }

      await executeShortcutAction(matchedShortcut, keyCombo)
    }
  }

  const findMatchingShortcut = (event: KeyboardEvent, config: ShortcutsConfig): ShortcutBinding | null => {
    for (const shortcut of config) {
      if (isShortcutMatch(event, shortcut)) {
        return shortcut
      }
    }

    return null
  }

  const executeShortcutAction = async (shortcut: ShortcutBinding, keyCombo: string) => {
    const actionName = extractActionName(shortcut.action)
    let frontendResult = false

    switch (actionName) {
      case 'copy_to_clipboard':
        frontendResult = await shortcutActionsService.copyToClipboard()
        break
      case 'paste_from_clipboard':
        frontendResult = await shortcutActionsService.pasteFromClipboard()
        break
      case 'command_palette':
        frontendResult = shortcutActionsService.commandPalette()
        break
      case 'accept_completion':
        frontendResult = shortcutActionsService.acceptCompletion()
        break
      case 'terminal_search':
        frontendResult = shortcutActionsService.terminalSearch()
        break
      case 'open_settings':
        frontendResult = shortcutActionsService.openSettings()
        break
      case 'new_terminal':
        frontendResult = shortcutActionsService.newTerminal()
        break
      case 'clear_terminal':
        frontendResult = shortcutActionsService.clearTerminal()
        break
      case 'toggle_terminal_panel':
        frontendResult = shortcutActionsService.toggleTerminalPanel()
        break
      case 'toggle_window_pin':
        frontendResult = await shortcutActionsService.toggleWindowPin()
        break
    }

    await shortcutsApi.executeAction(shortcut.action, keyCombo, getCurrentTerminalId(), {
      timestamp: new Date().toISOString(),
      userAgent: navigator.userAgent,
      frontendResult,
    })

    return frontendResult
  }

  const getCurrentTerminalId = (): string | null => {
    return typeof terminalStore.activeTerminalId === 'number' ? String(terminalStore.activeTerminalId) : null
  }

  const reloadConfig = async () => {
    config.value = await shortcutsApi.getConfig()
  }

  const stopListener = () => {
    if (keydownHandler) {
      document.removeEventListener('keydown', keydownHandler, true)
      keydownHandler = null
    }
    isListening.value = false
  }

  onMounted(() => {
    initializeListener()
  })

  onUnmounted(() => {
    stopListener()
  })

  return {
    isListening,
    config,
    reloadConfig,
  }
}
