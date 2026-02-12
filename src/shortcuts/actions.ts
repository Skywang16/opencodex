import { windowApi } from '@/api/window'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useLayoutStore } from '@/stores/layout'
import { useTerminalStore } from '@/stores/Terminal'
import { useWindowStore } from '@/stores/Window'

export class ShortcutActionsService {
  private get layoutStore() {
    return useLayoutStore()
  }

  copyToClipboard = async (): Promise<boolean> => {
    return true
  }

  pasteFromClipboard = async (): Promise<boolean> => {
    return true
  }

  commandPalette = (): boolean => {
    document.dispatchEvent(new CustomEvent('toggle-command-palette'))
    return true
  }

  terminalSearch = (): boolean => {
    document.dispatchEvent(new CustomEvent('open-terminal-search'))
    return true
  }

  acceptCompletion = (): boolean => {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('accept-completion', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  openSettings = (): boolean => {
    this.layoutStore.openSettings()
    return true
  }

  newTerminal = (): boolean => {
    const terminalStore = useTerminalStore()
    const layoutStore = useLayoutStore()
    terminalStore.createTerminalPane()
    layoutStore.openTerminalPanel()
    return true
  }

  clearTerminal = (): boolean => {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('clear-terminal', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  toggleTerminalPanel = (): boolean => {
    this.layoutStore.toggleTerminalPanel()
    return true
  }

  toggleAISidebar = async (): Promise<boolean> => {
    const aiChatStore = useAIChatStore()
    await aiChatStore.toggleSidebar()
    return true
  }

  toggleWindowPin = async (): Promise<boolean> => {
    const newState = await windowApi.toggleAlwaysOnTop()
    const windowStore = useWindowStore()
    windowStore.setAlwaysOnTop(newState)
    return true
  }
}

export const shortcutActionsService = new ShortcutActionsService()
