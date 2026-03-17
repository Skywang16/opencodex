<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar/store'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Block } from '@/types'
  import { computed, nextTick, ref, watch } from 'vue'
  import AIMessage from '../AIMessage.vue'

  type SubtaskBlock = Extract<Block, { type: 'subtask' }>

  interface Props {
    block: SubtaskBlock
  }

  const props = defineProps<Props>()
  const aiChatStore = useAIChatStore()
  const workspaceStore = useWorkspaceStore()
  const formatErrorMessage = (error: unknown): string => {
    return error instanceof Error ? error.message : String(error)
  }

  const expanded = ref(false)
  const userToggled = ref(false)
  const loadError = ref<string | null>(null)
  const resultWrapperRef = ref<HTMLElement | null>(null)
  const canScroll = ref(false)
  const showTopFade = ref(false)
  const showBottomFade = ref(false)

  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')
  const isError = computed(() => props.block.status === 'error')
  const isCancelled = computed(() => props.block.status === 'cancelled')

  const aiMessages = computed(() => {
    const messages = workspaceStore.getCachedMessages(props.block.childSessionId)
    return messages.filter(msg => msg.role === 'assistant')
  })

  const hasContent = computed(() => aiMessages.value.length > 0 || isRunning.value)
  const isWaitingForContent = computed(() => isRunning.value && aiMessages.value.length === 0)

  const displayTitle = computed(() => {
    const title = props.block.agentType || 'agent'
    return title.charAt(0).toUpperCase() + title.slice(1)
  })

  const statusTone = computed(() => {
    if (isError.value) return 'error'
    if (isCancelled.value) return 'cancelled'
    if (isRunning.value) return 'running'
    return 'completed'
  })

  const previewText = computed(() => {
    if (props.block.summary?.trim()) return props.block.summary.trim()
    if (isWaitingForContent.value) return 'Waiting for first streamed output…'
    if (isRunning.value) return 'Streaming task progress…'
    if (isError.value) return 'Child agent ended with an error.'
    if (isCancelled.value) return 'Child agent execution was cancelled.'
    return 'No summary yet.'
  })

  const updateScrollFades = () => {
    const el = resultWrapperRef.value
    if (!el) {
      canScroll.value = false
      showTopFade.value = false
      showBottomFade.value = false
      return
    }

    const scrollable = el.scrollHeight - el.clientHeight > 8
    canScroll.value = scrollable
    showTopFade.value = scrollable && el.scrollTop > 6
    showBottomFade.value = scrollable && el.scrollTop + el.clientHeight < el.scrollHeight - 6
  }

  const ensureMessagesLoaded = async () => {
    const cachedMessages = workspaceStore.getCachedMessages(props.block.childSessionId)
    if (cachedMessages.length > 0) return

    loadError.value = null
    try {
      await workspaceStore.fetchMessages(props.block.childSessionId)
    } catch (err) {
      console.warn(`Failed to load subtask session ${props.block.childSessionId} messages:`, err)
      loadError.value = formatErrorMessage(err)
    }
  }

  const toggleDetails = async () => {
    if (!hasContent.value) return
    userToggled.value = true
    expanded.value = !expanded.value
    if (!expanded.value) return

    await ensureMessagesLoaded()
    await nextTick()
    updateScrollFades()
  }

  const openDetails = async () => {
    await aiChatStore.switchSession(props.block.childSessionId)
  }

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

  watch(
    () => [expanded.value, aiMessages.value.length, props.block.summary, props.block.status],
    async () => {
      await nextTick()
      updateScrollFades()
    }
  )
</script>

<template>
  <div class="subtask-block">
    <div class="subtask-card" :class="{ running: isRunning, error: isError, cancelled: isCancelled }">
      <div class="subtask-card-top">
        <div class="subtask-card-meta">
          <span class="subtask-prefix">{{ displayTitle }}</span>
          <span class="subtask-status-dot" :class="statusTone" aria-hidden="true"></span>
        </div>
        <button
          v-if="hasContent"
          type="button"
          class="subtask-toggle"
          :aria-expanded="expanded"
          :aria-label="expanded ? 'Collapse child agent output' : 'Expand child agent output'"
          @click="toggleDetails"
        >
          <svg class="chevron" :class="{ expanded }" width="10" height="10" viewBox="0 0 10 10">
            <path
              d="M3.5 2.5L6 5L3.5 7.5"
              stroke="currentColor"
              stroke-width="1"
              stroke-linecap="round"
              stroke-linejoin="round"
              fill="none"
            />
          </svg>
        </button>
      </div>

      <div class="subtask-card-main">
        <div class="subtask-description">{{ block.description }}</div>
        <div class="subtask-preview" :class="{ running: isRunning }">{{ previewText }}</div>
      </div>

      <transition name="expand">
        <div v-if="expanded" class="subtask-result" @click.stop>
          <div class="subtask-result-shell">
            <div v-if="showTopFade" class="scroll-fade top"></div>
            <div v-if="showBottomFade" class="scroll-fade bottom"></div>

            <div ref="resultWrapperRef" class="subtask-scroll" @scroll="updateScrollFades">
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
              </div>
              <div v-else class="subtask-empty">No output</div>
            </div>
          </div>
        </div>
      </transition>

      <div class="subtask-card-footer">
        <button type="button" class="subtask-detail-link" @click.stop="openDetails">Open</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .subtask-block {
    min-width: 0;
  }

  .subtask-card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 100%;
    padding: 10px 11px;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-200) 82%, transparent);
    background: transparent;
    box-shadow: none;
  }

  .subtask-card.running {
    border-color: color-mix(in srgb, var(--color-primary) 26%, var(--border-200));
  }

  .subtask-card.error {
    border-color: color-mix(in srgb, var(--color-error) 45%, var(--border-200));
  }

  .subtask-card.cancelled {
    opacity: 0.86;
  }

  .subtask-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .subtask-card-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .subtask-prefix {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-200) 78%, transparent);
    background: transparent;
    color: var(--text-300);
    font-size: 11px;
    font-weight: 600;
  }

  .subtask-status-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--text-500);
    flex-shrink: 0;
  }

  .subtask-status-dot.running {
    background: var(--color-primary);
  }

  .subtask-status-dot.error {
    background: var(--color-error);
  }

  .subtask-status-dot.cancelled {
    background: var(--text-500);
    opacity: 0.7;
  }

  .subtask-status-dot.completed {
    background: color-mix(in srgb, var(--color-success) 78%, var(--text-500));
  }

  .subtask-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: 1px solid color-mix(in srgb, var(--border-200) 72%, transparent);
    border-radius: 999px;
    padding: 0;
    background: transparent;
    color: var(--text-400);
    cursor: pointer;
  }

  .subtask-toggle:hover {
    color: var(--text-200);
    border-color: color-mix(in srgb, var(--border-200) 92%, transparent);
  }

  .subtask-card-main {
    min-width: 0;
  }

  .subtask-description {
    color: var(--text-200);
    font-size: 13px;
    line-height: 1.35;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .subtask-preview {
    color: var(--text-500);
    font-size: 12px;
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    min-height: calc(1.45em * 2);
  }

  .subtask-preview.running {
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

  .chevron {
    flex-shrink: 0;
    color: currentColor;
    transition: transform 0.2s ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .subtask-result {
    margin-top: 8px;
  }

  .subtask-result-shell {
    position: relative;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--border-200) 68%, transparent);
    background: transparent;
  }

  .subtask-scroll {
    max-height: 220px;
    overflow-y: auto;
    padding: 8px 10px 10px;
  }

  .scroll-fade {
    position: absolute;
    left: 0;
    right: 0;
    height: 28px;
    z-index: 2;
    pointer-events: none;
  }

  .scroll-fade.top {
    top: 0;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-100) 98%, transparent), transparent);
  }

  .scroll-fade.bottom {
    bottom: 0;
    background: linear-gradient(0deg, color-mix(in srgb, var(--bg-100) 98%, transparent), transparent);
  }

  .subtask-loading {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 4px 0;
    color: var(--text-400);
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
    color: var(--color-error);
    font-size: 13px;
    white-space: pre-wrap;
  }

  .subtask-empty {
    color: var(--text-500);
    font-size: 12px;
  }

  .subtask-messages {
    padding-top: 2px;
  }

  .subtask-card-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 8px;
  }

  .subtask-detail-link {
    border: none;
    background: transparent;
    padding: 0;
    color: var(--color-primary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .subtask-detail-link:hover {
    color: color-mix(in srgb, var(--color-primary) 72%, white);
  }
</style>
