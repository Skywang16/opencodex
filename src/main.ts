import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useLayoutStore } from '@/stores/layout'
import { useThemeStore } from '@/stores/theme'

import { useTerminalStore } from '@/stores/Terminal'
import { useFileWatcherStore } from '@/stores/fileWatcher'
import { openUrl } from '@tauri-apps/plugin-opener'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'

import { i18n, initLocale } from './i18n'
// Fontsource - self-hosted fonts (no network required)
import '@fontsource/inter/400.css'
import '@fontsource/inter/500.css'
import '@fontsource/inter/600.css'
import '@fontsource/inter/700.css'
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import './styles/variables.css'
import ui from './ui'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(ui)
app.use(i18n)

const initializeApplication = async () => {
  try {
    const themeStore = useThemeStore()
    const layoutStore = useLayoutStore()
    const aiChatStore = useAIChatStore()
    const aiSettingsStore = useAISettingsStore()
    const terminalStore = useTerminalStore()
    const fileWatcherStore = useFileWatcherStore()

    // Phase 1: initialize things that affect initial render
    await Promise.allSettled([themeStore.initialize(), initLocale(), layoutStore.initialize()])

    // Phase 2: initialize core stores (avoid duplicate init calls / race conditions)
    await aiChatStore.initialize().catch(error => {
      console.warn('AI chat store initialization failed:', error)
    })

    app.mount('#app')

    // Phase 3: background initialization (doesn't block first paint)
    await Promise.allSettled([
      aiSettingsStore.loadSettings(),
      terminalStore.initializeTerminalStore(),
      fileWatcherStore.initialize(),
    ])
  } catch (error) {
    console.error('Error during app initialization:', error)
    if (!document.getElementById('app')?.hasChildNodes()) {
      app.mount('#app')
    }
  }
}

initializeApplication()

const disableContextMenuInProduction = () => {
  if (import.meta.env.PROD) {
    document.addEventListener('contextmenu', event => {
      event.preventDefault()
      return false
    })

    document.addEventListener('keydown', event => {
      if (
        event.key === 'F12' ||
        (event.ctrlKey && event.shiftKey && event.key === 'I') ||
        (event.ctrlKey && event.shiftKey && event.key === 'C') ||
        (event.ctrlKey && event.key === 'U')
      ) {
        event.preventDefault()
        return false
      }
    })
  }
}

disableContextMenuInProduction()

// Intercept external link clicks globally, open in system browser
const setupExternalLinkHandler = () => {
  document.addEventListener('click', (e: MouseEvent) => {
    const target = e.target as HTMLElement
    const link = target.closest('a[href]') as HTMLAnchorElement | null

    if (link) {
      const href = link.getAttribute('href')
      // Skip internal anchor links and javascript: links
      if (!href || href.startsWith('#') || href.startsWith('javascript:')) {
        return
      }
      // Open external links in system browser
      if (href.startsWith('http://') || href.startsWith('https://')) {
        e.preventDefault()
        openUrl(href).catch(err => {
          console.error('Failed to open URL:', err)
        })
      }
    }
  })
}

setupExternalLinkHandler()
