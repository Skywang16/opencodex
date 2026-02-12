<script setup lang="ts">
  import { appApi, workspaceApi } from '@/api'
  import { useMenuEvents } from '@/composables/useMenuEvents'
  import { useShortcutListener } from '@/shortcuts'
  import MainView from '@/views/MainView.vue'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import { onMounted, onUnmounted } from 'vue'

  const { reloadConfig } = useShortcutListener()

  // Initialize menu event listeners
  useMenuEvents()

  let unlistenClearTabs: UnlistenFn | undefined

  onMounted(async () => {
    ;(window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts = reloadConfig

    // Maintain workspace data in background
    workspaceApi.maintainWorkspaces()

    // Listen for clear all tabs event (triggered when macOS window closes)
    unlistenClearTabs = await appApi.onClearAllTabs(async () => {
      // No-op now that tabs are removed
    })
  })

  onUnmounted(() => {
    if (unlistenClearTabs) {
      unlistenClearTabs()
    }
  })
</script>

<template>
  <div class="app-layout">
    <MainView />
  </div>
</template>

<style>
  :root {
    font-family: var(--font-family);
    font-size: var(--font-size-lg);
    font-weight: 400;
    color: var(--text-300);
    background-color: transparent;

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }

  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  body,
  html {
    height: 100%;
    overflow: hidden;
    background: transparent !important;
  }

  #app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: transparent;
  }

  .app-layout {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: transparent;
    position: relative;
  }
</style>
