<script setup lang="ts">
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Block } from '@/types'
  import { computed, ref, watch } from 'vue'
  import AIMessage from '../AIMessage.vue'

  type SubtaskBlock = Extract<Block, { type: 'subtask' }>

  interface Props {
    block: SubtaskBlock
  }

  const props = defineProps<Props>()
  const expanded = ref(false)
  const userToggled = ref(false)
  const workspaceStore = useWorkspaceStore()
  const loadError = ref<string | null>(null)

  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')
  const isCompleted = computed(() => props.block.status === 'completed')
  const isError = computed(() => props.block.status === 'error')
  const isCancelled = computed(() => props.block.status === 'cancelled')

  // Only get assistant messages, filter out user messages (prompts)
  const aiMessages = computed(() => {
    const messages = workspaceStore.getCachedMessages(props.block.childSessionId)
    return messages.filter(msg => msg.role === 'assistant')
  })

  // Whether there is content that can be expanded to view
  const hasContent = computed(() => {
    return aiMessages.value.length > 0 || isRunning.value
  })

  // Whether waiting for the first message (running but no AI messages yet)
  const isWaitingForContent = computed(() => {
    return isRunning.value && aiMessages.value.length === 0
  })

  const displayTitle = computed(() => {
    const title = props.block.agentType || 'agent'
    return title.charAt(0).toUpperCase() + title.slice(1)
  })

  const toggleDetails = async () => {
    if (!hasContent.value) return
    userToggled.value = true
    expanded.value = !expanded.value
    if (!expanded.value) return

    // Try to load historical messages when expanding (if cache is empty)
    const cachedMessages = workspaceStore.getCachedMessages(props.block.childSessionId)
    if (cachedMessages.length === 0) {
      loadError.value = null
      try {
        await workspaceStore.fetchMessages(props.block.childSessionId)
      } catch (err) {
        loadError.value = err instanceof Error ? err.message : String(err)
      }
    }
  }

  // Auto expand when running, keep expanded state after completion
  watch(
    () => props.block.status,
    status => {
      if (userToggled.value) return
      if (status === 'running' || status === 'pending') {
        expanded.value = true
      }
    },
    { immediate: true }
  )
</script>

<template>
  <div class="subtask-block">
    <div
      class="subtask-line"
      :class="{ clickable: hasContent, running: isRunning, error: isError, cancelled: isCancelled }"
      @click="toggleDetails"
    >
      <span class="text" :class="{ running: isRunning }">
        <span class="subtask-prefix">{{ displayTitle }}</span>
        <span class="subtask-description">{{ block.description }}</span>
      </span>
      <svg v-if="hasContent" class="chevron" :class="{ expanded }" width="10" height="10" viewBox="0 0 10 10">
        <path
          d="M3.5 2.5L6 5L3.5 7.5"
          stroke="currentColor"
          stroke-width="1"
          stroke-linecap="round"
          stroke-linejoin="round"
          fill="none"
        />
      </svg>
    </div>

    <transition name="expand">
      <div v-if="expanded" class="subtask-result" @click.stop>
        <div v-if="loadError" class="subtask-error">{{ loadError }}</div>
        <template v-else-if="aiMessages.length > 0">
          <div class="subtask-messages">
            <template v-for="msg in aiMessages" :key="msg.id">
              <AIMessage :message="msg" :disable-tool-expand="true" />
            </template>
          </div>
        </template>
        <div v-else-if="isWaitingForContent" class="subtask-loading">
          <span class="loading-dot"></span>
          <span>Running…</span>
        </div>
        <div v-else class="subtask-empty">No output</div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
  .subtask-block {
    margin: 6px 0;
    font-size: 14px;
    line-height: 1.8;
  }

  .subtask-line {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-400);
    transition: all 0.15s ease;
  }

  .subtask-line.clickable {
    cursor: pointer;
  }

  .subtask-line.clickable:hover {
    color: var(--text-300);
  }

  .subtask-line.clickable:hover .chevron {
    opacity: 1;
  }

  .subtask-line.running .text,
  .text.running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  @keyframes scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .subtask-line.error {
    color: var(--color-error);
  }

  .subtask-line.cancelled {
    color: var(--text-500);
    opacity: 0.85;
  }

  .text {
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .subtask-prefix {
    color: var(--text-400);
    font-weight: 500;
  }

  .subtask-description {
    color: var(--text-500);
    font-weight: 400;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    opacity: 0.5;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .subtask-result {
    margin-top: 2px;
    max-height: 300px;
    overflow: hidden;
  }

  .subtask-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    color: var(--text-400);
    font-size: 13px;
  }

  .loading-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .subtask-error {
    padding: 8px 0;
    color: var(--color-error);
    font-size: 13px;
    white-space: pre-wrap;
  }

  .subtask-empty {
    padding: 8px 0;
    color: var(--text-500);
    font-size: 13px;
  }

  .subtask-messages {
    padding-top: 4px;
  }

  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.2s ease;
    overflow: hidden;
  }

  .expand-enter-from,
  .expand-leave-to {
    max-height: 0;
    opacity: 0;
    margin-top: 0;
  }

  .expand-enter-to,
  .expand-leave-from {
    max-height: 300px;
    opacity: 1;
    margin-top: 2px;
  }

  .subtask-messages :deep(.ai-message) {
    margin-bottom: var(--spacing-sm);
  }

  .subtask-messages :deep(.ai-message-footer) {
    display: none;
  }
</style>
