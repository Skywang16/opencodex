import { workspaceApi } from '@/api/workspace'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useLayoutStore = defineStore('layout', () => {
  // Persisted UI state
  const sidebarVisible = ref(true)
  const sidebarWidth = ref(260)
  const terminalPanelVisible = ref(false)
  const terminalPanelHeight = ref(200)
  const aiSidebarVisible = ref(false)
  const aiSidebarWidth = ref(350)
  const selectedModelId = ref<string | null>(null)

  // Non-persisted state
  const showSettings = ref(false)
  const initialized = ref(false)
  const dragPath = ref<string | null>(null)

  // Initialize: batch read from app_preferences
  const initialize = async () => {
    if (initialized.value) return
    try {
      const prefs = await workspaceApi.getPreferences([
        'ui.sidebar_visible',
        'ui.sidebar_width',
        'ui.terminal_visible',
        'ui.terminal_height',
        'ui.ai_sidebar_visible',
        'ui.ai_sidebar_width',
        'ui.selected_model_id',
      ])

      if (prefs['ui.sidebar_visible'] !== undefined) sidebarVisible.value = prefs['ui.sidebar_visible'] === 'true'
      if (prefs['ui.sidebar_width'] !== undefined) sidebarWidth.value = Number(prefs['ui.sidebar_width']) || 260
      if (prefs['ui.terminal_visible'] !== undefined)
        terminalPanelVisible.value = prefs['ui.terminal_visible'] === 'true'
      if (prefs['ui.terminal_height'] !== undefined)
        terminalPanelHeight.value = Number(prefs['ui.terminal_height']) || 200
      if (prefs['ui.ai_sidebar_visible'] !== undefined)
        aiSidebarVisible.value = prefs['ui.ai_sidebar_visible'] === 'true'
      if (prefs['ui.ai_sidebar_width'] !== undefined) aiSidebarWidth.value = Number(prefs['ui.ai_sidebar_width']) || 350
      if (prefs['ui.selected_model_id'] !== undefined) selectedModelId.value = prefs['ui.selected_model_id'] || null
    } catch (e) {
      console.warn('Failed to load layout preferences:', e)
    }
    initialized.value = true
  }

  const persist = (key: string, value: string | null) => {
    if (initialized.value) {
      workspaceApi.setPreference(key, value).catch(e => {
        console.warn(`Failed to persist ${key}:`, e)
      })
    }
  }

  // Setters with persistence
  const setSidebarVisible = (v: boolean) => {
    sidebarVisible.value = v
    persist('ui.sidebar_visible', String(v))
  }

  const setSidebarWidth = (w: number) => {
    sidebarWidth.value = w
    persist('ui.sidebar_width', String(w))
  }

  const setTerminalPanelHeight = (h: number) => {
    terminalPanelHeight.value = h
    persist('ui.terminal_height', String(h))
  }

  const setAiSidebarVisible = (v: boolean) => {
    aiSidebarVisible.value = v
    persist('ui.ai_sidebar_visible', String(v))
  }

  const setAiSidebarWidth = (w: number) => {
    aiSidebarWidth.value = Math.max(300, Math.min(800, w))
    persist('ui.ai_sidebar_width', String(aiSidebarWidth.value))
  }

  const setSelectedModelId = (id: string | null) => {
    selectedModelId.value = id
    persist('ui.selected_model_id', id)
  }

  // Settings page
  const openSettings = () => {
    showSettings.value = true
  }

  const closeSettings = () => {
    showSettings.value = false
  }

  // Terminal panel toggle
  const toggleTerminalPanel = () => {
    const v = !terminalPanelVisible.value
    terminalPanelVisible.value = v
    persist('ui.terminal_visible', String(v))
  }

  const openTerminalPanel = () => {
    terminalPanelVisible.value = true
    persist('ui.terminal_visible', 'true')
  }

  const closeTerminalPanel = () => {
    terminalPanelVisible.value = false
    persist('ui.terminal_visible', 'false')
  }

  // Drag path
  const setDragPath = (path: string | null) => {
    dragPath.value = path
  }

  const consumeDragPath = () => {
    const path = dragPath.value
    dragPath.value = null
    return path
  }

  return {
    // State
    sidebarVisible,
    sidebarWidth,
    terminalPanelVisible,
    terminalPanelHeight,
    aiSidebarVisible,
    aiSidebarWidth,
    selectedModelId,
    showSettings,
    initialized,
    dragPath,

    // Init
    initialize,

    // Setters
    setSidebarVisible,
    setSidebarWidth,
    setTerminalPanelHeight,
    setAiSidebarVisible,
    setAiSidebarWidth,
    setSelectedModelId,

    // Settings
    openSettings,
    closeSettings,

    // Terminal panel
    toggleTerminalPanel,
    openTerminalPanel,
    closeTerminalPanel,

    // Drag
    setDragPath,
    consumeDragPath,
  }
})
