<script setup lang="ts">
  /**
   * ChatHeader - Header of the chat sidebar
   * Note: This component is only used in AIChatSidebar, the main interface uses MainChatArea
   */
  import type { SessionRecord } from '@/api/workspace'
  import { useI18n } from 'vue-i18n'
  import SessionSelect from './SessionSelect.vue'

  interface Props {
    sessions: SessionRecord[]
    currentSessionId?: number | null
    isLoading?: boolean
  }

  interface Emits {
    (e: 'select-session', sessionId: number): void
    (e: 'create-new-session'): void
    (e: 'refresh-sessions'): void
  }

  withDefaults(defineProps<Props>(), {
    isLoading: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const handleSelectSession = (sessionId: number) => {
    emit('select-session', sessionId)
  }

  const handleCreateNewSession = () => {
    emit('create-new-session')
  }

  const handleRefreshSessions = () => {
    emit('refresh-sessions')
  }
</script>

<template>
  <div class="chat-header">
    <div class="header-content">
      <SessionSelect
        :sessions="sessions"
        :current-session-id="currentSessionId || null"
        :loading="isLoading"
        @select-session="handleSelectSession"
        @create-new-session="handleCreateNewSession"
        @refresh-sessions="handleRefreshSessions"
      />
    </div>

    <div class="header-actions">
      <button class="icon-btn" :title="t('chat.new_session')" @click="handleCreateNewSession">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .chat-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border-200);
    background-color: var(--bg-200);
    padding: 6px 12px 0 12px;
    gap: 8px;
    height: 40px;
    position: relative;
  }

  .chat-header::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: -24px;
    height: 24px;
    pointer-events: none;
    z-index: -1;
    background: linear-gradient(to bottom, var(--bg-200) 0%, transparent 100%);
  }

  .header-content {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }
</style>
