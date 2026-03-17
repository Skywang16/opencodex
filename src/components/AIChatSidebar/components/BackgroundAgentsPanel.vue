<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar/store'
  import { getAgentColor } from '@/utils/agentColors'
  import { ref } from 'vue'

  const aiChatStore = useAIChatStore()
  const isCollapsed = ref(false)

  const truncate = (s: string, n: number) => (s.length > n ? s.slice(0, n) + '…' : s)

  const getStatusText = (
    status: 'queued' | 'running' | 'completed' | 'cancelled' | 'error',
  ) => {
    switch (status) {
      case 'running':
        return 'is thinking'
      case 'queued':
        return 'is awaiting instruction'
      case 'completed':
        return 'completed'
      case 'cancelled':
        return 'cancelled'
      case 'error':
        return 'failed'
    }
  }

  const handleOpen = (sessionId: number) => {
    aiChatStore.switchSession(sessionId)
  }
</script>

<template>
  <div v-if="aiChatStore.activeExecutionNodes.length > 0" class="background-agents-panel">
    <div class="panel-header" @click="isCollapsed = !isCollapsed">
      <svg class="panel-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <circle cx="8" cy="8" r="6" />
        <path d="M8 5v3l2 1" stroke-linecap="round" />
      </svg>
      <span class="panel-title">{{ aiChatStore.activeExecutionNodes.length }} active branches</span>
      <svg
        class="collapse-icon"
        :class="{ collapsed: isCollapsed }"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
      >
        <path d="M4 10l4-4 4 4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </div>

    <Transition name="panel-collapse">
      <div v-if="!isCollapsed" class="panel-body">
        <div
          v-for="(node, idx) in aiChatStore.activeExecutionNodes"
          :key="node.nodeId"
          class="agent-row"
        >
          <span class="agent-dot" :style="{ background: getAgentColor(idx) }" />
          <span class="agent-name" :style="{ color: getAgentColor(idx) }">
            {{ truncate(node.title, 20) }}
          </span>
          <span class="agent-status">{{ node.profile }} · {{ getStatusText(node.status) }}</span>
          <button class="open-btn" @click="handleOpen(node.backingSessionId)">Open</button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
  .background-agents-panel {
    border-top: 1px solid var(--border-200);
    background: var(--bg-100-solid);
    flex-shrink: 0;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
  }

  .panel-header:hover {
    background: var(--color-hover);
  }

  .panel-icon {
    width: 14px;
    height: 14px;
    color: var(--text-300);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-300);
    flex: 1;
  }

  .collapse-icon {
    width: 14px;
    height: 14px;
    color: var(--text-400);
    flex-shrink: 0;
    transition: transform 0.15s ease;
  }

  .collapse-icon.collapsed {
    transform: rotate(180deg);
  }

  .panel-body {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .agent-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 16px;
  }

  .agent-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .agent-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
    flex-shrink: 0;
  }

  .agent-status {
    font-size: 12px;
    color: var(--text-400);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open-btn {
    font-size: 12px;
    color: var(--text-300);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--border-radius-sm);
    flex-shrink: 0;
  }

  .open-btn:hover {
    color: var(--text-100);
    background: var(--color-hover);
  }

  /* Collapse animation */
  .panel-collapse-enter-active,
  .panel-collapse-leave-active {
    transition:
      max-height 0.2s ease,
      opacity 0.2s ease;
    max-height: 300px;
  }

  .panel-collapse-enter-from,
  .panel-collapse-leave-to {
    max-height: 0;
    opacity: 0;
  }
</style>
