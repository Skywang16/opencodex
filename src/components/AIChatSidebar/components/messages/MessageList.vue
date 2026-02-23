<script setup lang="ts">
  import { useAISettingsStore } from '@/components/settings/components/AI/store'
  import { useCheckpoint } from '@/composables/useCheckpoint'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Message } from '@/types'
  import { showPopoverAt } from '@/ui'
  import { formatRelativeTime } from '@/utils/dateFormatter'
  import { computed, nextTick, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from '../../store'
  import AIMessage from './AIMessage.vue'
  import UserMessage from './UserMessage.vue'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()
  const aiChatStore = useAIChatStore()
  const workspaceStore = useWorkspaceStore()
  const { loadCheckpoints, getCheckpointByMessageId } = useCheckpoint()

  interface Props {
    messages: Message[]
    isLoading?: boolean
    sessionId?: number | null
    workspacePath?: string
  }

  const props = defineProps<Props>()

  // Get the last 3 historical sessions for the current workspace (excluding the current session)
  const recentSessions = computed(() => {
    const path = props.workspacePath || workspaceStore.currentWorkspacePath
    const node = workspaceStore.getNode(path)
    if (!node) return []
    return node.sessions.filter(s => s.id !== props.sessionId).slice(0, 3)
  })

  // Current workspace display name
  const currentWorkspaceName = computed(() => {
    const path = props.workspacePath || workspaceStore.currentWorkspacePath
    if (!path) return 'OpenCodex'
    const parts = path.split('/').filter(Boolean)
    return parts[parts.length - 1] || 'OpenCodex'
  })

  // Workspace dropdown
  const handleWorkspaceDropdown = async (event: MouseEvent) => {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const workspaces = workspaceStore.workspaces

    const items = [
      {
        label: t('header.open_folder'),
        onClick: async () => {
          const { open } = await import('@tauri-apps/plugin-dialog')
          const selected = await open({
            directory: true,
            multiple: false,
            title: t('header.select_folder'),
          })
          if (selected && typeof selected === 'string') {
            await workspaceStore.loadTree()
            await workspaceStore.loadSessions(selected)
          }
        },
      },
      ...(workspaces.length > 0
        ? [
            { label: '─────', disabled: true, onClick: () => {} },
            ...workspaces.slice(0, 8).map(ws => ({
              label: ws.displayName || ws.path.split('/').pop() || ws.path,
              onClick: async () => {
                await workspaceStore.loadSessions(ws.path)
              },
            })),
          ]
        : []),
    ]

    await showPopoverAt(rect.left, rect.bottom + 4, items)
  }

  const handleSelectSession = (sessionId: number) => {
    aiChatStore.switchSession(sessionId)
  }

  const messageListRef = ref<HTMLElement | null>(null)
  const isLoadingMore = ref(false)

  const scrollToBottom = async () => {
    await nextTick()
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight
    }
  }

  // Load older messages when scrolled near top, preserve scroll position
  const onScroll = async () => {
    const el = messageListRef.value
    if (!el || isLoadingMore.value || !workspaceStore.messagesHasMore) return
    if (el.scrollTop > 100) return

    isLoadingMore.value = true
    const prevHeight = el.scrollHeight
    try {
      await workspaceStore.loadMoreMessages()
      await nextTick()
      // Restore scroll position: new content pushed old content down
      el.scrollTop = el.scrollHeight - prevHeight
    } finally {
      isLoadingMore.value = false
    }
  }

  // Get checkpoint for the message (using message.id to find)
  const getCheckpoint = (message: Message) => {
    if (!props.sessionId || !props.workspacePath || message.role !== 'user') return null
    return getCheckpointByMessageId(props.sessionId, props.workspacePath, message.id)
  }

  // New messages appended at bottom → scroll to bottom
  watch(
    () => props.messages.length,
    async (newLength, oldLength) => {
      if (newLength <= oldLength) return
      await scrollToBottom()

      const last = props.messages[newLength - 1]
      if (last?.role === 'user' && props.sessionId && props.sessionId > 0 && props.workspacePath) {
        await loadCheckpoints(props.sessionId, props.workspacePath)
      }
    }
  )

  // Session switch → reload checkpoints + scroll to bottom
  watch(
    () => [props.sessionId, props.workspacePath] as const,
    async ([newId, workspacePath]) => {
      if (newId && newId > 0 && workspacePath) {
        await loadCheckpoints(newId, workspacePath)
      }
      await scrollToBottom()
    },
    { immediate: true }
  )

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
    await scrollToBottom()
  })
</script>

<template>
  <div ref="messageListRef" class="message-list" @scroll="onScroll">
    <div v-if="messages.length === 0" class="empty-state">
      <!-- No model configured -->
      <div v-if="!aiSettingsStore.hasModels && aiSettingsStore.isInitialized" class="no-model-state">
        <div class="empty-text">{{ t('message_list.no_model_configured') }}</div>
        <div class="empty-hint">{{ t('message_list.configure_model_hint') }}</div>
      </div>

      <!-- Codex-style welcome -->
      <div v-else class="welcome-state">
        <!-- Logo -->
        <div class="welcome-logo">
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle cx="24" cy="24" r="22" stroke="currentColor" stroke-width="1.5" opacity="0.25" />
            <circle cx="24" cy="24" r="14" stroke="currentColor" stroke-width="1.5" opacity="0.4" />
            <path
              d="M24 14 C28 18, 32 22, 32 26 C32 30.4, 28.4 34, 24 34 C19.6 34, 16 30.4, 16 26 C16 22, 20 18, 24 14Z"
              fill="currentColor"
              opacity="0.15"
            />
            <circle cx="24" cy="25" r="4" fill="currentColor" opacity="0.5" />
          </svg>
        </div>

        <!-- Title -->
        <h1 class="welcome-title">{{ t('message_list.lets_build') }}</h1>

        <!-- Workspace selector -->
        <button class="workspace-selector" @click="handleWorkspaceDropdown">
          <span class="workspace-selector-name">{{ currentWorkspaceName }}</span>
          <svg class="workspace-selector-chevron" width="12" height="12" viewBox="0 0 12 12">
            <path
              d="M3 4.5L6 7.5L9 4.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>

        <!-- Recent sessions as quick cards -->
        <div v-if="recentSessions.length > 0" class="quick-cards">
          <div
            v-for="session in recentSessions"
            :key="session.id"
            class="quick-card"
            @click="handleSelectSession(session.id)"
          >
            <span class="quick-card-title">{{ session.title }}</span>
            <span class="quick-card-time">{{ formatRelativeTime(session.updatedAt * 1000) }}</span>
          </div>
        </div>

        <!-- Explore more link -->
        <button v-if="recentSessions.length > 0" class="explore-more" @click="handleWorkspaceDropdown">
          {{ t('message_list.explore_more') }}
        </button>
      </div>
    </div>

    <div v-else class="message-container">
      <div v-if="isLoadingMore" class="loading-more">
        <span class="loading-more-spinner" />
      </div>
      <template v-for="message in messages" :key="message.id">
        <UserMessage
          v-if="message.role === 'user'"
          :message="message"
          :checkpoint="getCheckpoint(message)"
          :workspace-path="workspacePath"
        />
        <AIMessage v-else-if="message.role === 'assistant'" :message="message" />
      </template>
    </div>
  </div>
</template>

<style scoped>
  .message-list {
    flex: 1;
    overflow-y: auto;
    width: 100%;
    padding: var(--spacing-xl) 12.5% 64px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
  }

  .message-list::-webkit-scrollbar {
    width: 6px;
  }

  .message-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .message-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-xs);
  }

  .message-list::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    color: var(--text-400);
  }

  .no-model-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-lg);
  }

  .empty-text {
    font-size: var(--font-size-lg);
    font-weight: 500;
    color: var(--text-200);
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    color: var(--text-200);
  }

  /* Codex-style welcome state */
  .welcome-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0;
    padding-bottom: 80px;
  }

  .welcome-logo {
    color: var(--text-300);
    margin-bottom: 20px;
    opacity: 0.7;
  }

  .welcome-title {
    font-size: 28px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 8px 0;
    letter-spacing: -0.5px;
  }

  .workspace-selector {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: none;
    border: none;
    color: var(--text-300);
    font-size: 16px;
    font-weight: 400;
    cursor: pointer;
    border-radius: var(--border-radius-sm);
    transition: all 0.15s ease;
    margin-bottom: 32px;
  }

  .workspace-selector:hover {
    color: var(--text-200);
    background: var(--bg-200);
  }

  .workspace-selector-name {
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-selector-chevron {
    opacity: 0.5;
    flex-shrink: 0;
  }

  /* Quick cards */
  .quick-cards {
    display: flex;
    gap: 10px;
    max-width: 600px;
    width: 100%;
    flex-wrap: wrap;
    justify-content: center;
  }

  .quick-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px 16px;
    min-width: 160px;
    max-width: 200px;
    flex: 1;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
  }

  .quick-card:hover {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  .quick-card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
  }

  .quick-card-time {
    font-size: 11px;
    color: var(--text-500);
  }

  .explore-more {
    margin-top: 16px;
    padding: 4px 8px;
    background: none;
    border: none;
    color: var(--text-400);
    font-size: 13px;
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .explore-more:hover {
    color: var(--text-200);
  }

  .message-container {
    display: flex;
    flex-direction: column;
  }

  .message-container > :deep(*) {
    border-bottom: none;
  }

  .loading-more {
    display: flex;
    justify-content: center;
    padding: 12px 0;
  }

  .loading-more-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top-color: var(--text-300);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
