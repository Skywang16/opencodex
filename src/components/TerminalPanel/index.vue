<script setup lang="ts">
  /**
   * TerminalPanel - User terminal panel
   * Displays a real interactive terminal
   */
  import Terminal from '@/components/terminal/Terminal.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { computed, onMounted, ref, watch } from 'vue'

  const terminalStore = useTerminalStore()

  // User terminal state
  const userTerminalId = ref<number | null>(null)
  const isTerminalReady = ref(false)

  // Whether active terminal
  const isActive = computed(() => {
    return terminalStore.activeTerminalId === userTerminalId.value
  })

  // Initialize user terminal
  const initUserTerminal = async () => {
    // If terminal exists, use existing one
    if (terminalStore.terminals.length > 0) {
      userTerminalId.value = terminalStore.terminals[0].id
      await terminalStore.setActiveTerminal(userTerminalId.value)
      isTerminalReady.value = true
      return
    }
    // Otherwise create new terminal
    const id = await terminalStore.createTerminalPane()
    userTerminalId.value = id
    await terminalStore.setActiveTerminal(id)
    isTerminalReady.value = true
  }

  onMounted(async () => {
    await terminalStore.initializeTerminalStore()
    await initUserTerminal()
  })

  // Watch terminal list changes, ensure terminal is available
  watch(
    () => terminalStore.terminals,
    newTerminals => {
      if (newTerminals.length > 0 && userTerminalId.value === null) {
        userTerminalId.value = newTerminals[0].id
        terminalStore.setActiveTerminal(userTerminalId.value)
        isTerminalReady.value = true
      }
    },
    { deep: true }
  )
</script>

<template>
  <div class="terminal-panel">
    <!-- Terminal content -->
    <div class="terminal-content">
      <Terminal v-if="isTerminalReady && userTerminalId !== null" :terminal-id="userTerminalId" :is-active="isActive" />
      <div v-else class="loading-state">
        <span>Initializing terminal...</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-200);
    position: relative;
  }

  .terminal-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-400);
    font-size: 13px;
  }
</style>
