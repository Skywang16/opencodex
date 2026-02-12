import { shortcutActionsService } from '@/shortcuts/actions'
import { useLayoutStore } from '@/stores/layout'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted } from 'vue'

export const useMenuEvents = () => {
  const layoutStore = useLayoutStore()
  const unlisteners: UnlistenFn[] = []

  const menuHandlers: [string, () => void][] = [
    // Edit
    ['menu:find', () => shortcutActionsService.terminalSearch()],
    ['menu:clear-terminal', () => shortcutActionsService.clearTerminal()],

    // View
    ['menu:toggle-ai-sidebar', () => shortcutActionsService.toggleAISidebar()],

    // Window
    ['menu:toggle-always-on-top', () => shortcutActionsService.toggleWindowPin()],

    // Settings
    ['menu:preferences', () => layoutStore.openSettings()],
  ]

  onMounted(async () => {
    for (const [event, handler] of menuHandlers) {
      unlisteners.push(await listen(event, handler))
    }
  })

  onUnmounted(() => {
    unlisteners.forEach(fn => fn())
  })
}
