import { agentApi } from '@/api/agent'
import type { TaskProgressPayload, TaskProgressStream } from '@/api/agent/types'
import { useAISettingsStore } from '@/components/settings/components/AI'
import type { ImageAttachment } from '@/stores/imageLightbox'
import { useLayoutStore } from '@/stores/layout'
import { useToolConfirmationDialogStore } from '@/stores/toolConfirmationDialog'
import { useWorkspaceStore } from '@/stores/workspace'
import type { ChatMode, RetryStatus } from '@/types'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export interface QueuedMessage {
  id: string
  content: string
  images?: ImageAttachment[]
}

export const useAIChatStore = defineStore('ai-chat', () => {
  const workspaceStore = useWorkspaceStore()
  const layoutStore = useLayoutStore()
  const aiSettingsStore = useAISettingsStore()
  const toolConfirmStore = useToolConfirmationDialogStore()

  // UI State (delegated to layoutStore)
  const isVisible = computed(() => layoutStore.aiSidebarVisible)
  const sidebarWidth = computed(() => layoutStore.aiSidebarWidth)
  const chatMode = computed(() => layoutStore.aiChatMode)

  // Local UI state
  const isInitialized = ref(false)
  const error = ref<string | null>(null)
  const cancelFunction = ref<(() => void) | null>(null)
  const cancelRequested = ref(false)
  const contextUsage = ref<{ tokensUsed: number; contextWindow: number } | null>(null)
  const retryStatus = ref<RetryStatus | null>(null)
  const pendingCommandId = ref<string | null>(null)

  // Message queue: per-session, memory-only (no persistence)
  const messageQueueMap = ref<Map<number, QueuedMessage[]>>(new Map())
  const userCancelled = ref(false)

  // 获取当前会话的消息队列，无会话返回 null
  const getCurrentQueue = (): QueuedMessage[] | null => {
    const sid = currentSession.value?.id
    if (sid == null) return null
    let q = messageQueueMap.value.get(sid)
    if (!q) {
      q = []
      messageQueueMap.value.set(sid, q)
    }
    return q
  }

  const currentSessionQueue = computed(() => getCurrentQueue() ?? [])

  const enqueueMessage = (content: string, images?: ImageAttachment[]) => {
    getCurrentQueue()?.push({ id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, content, images })
  }

  const removeQueuedMessage = (messageId: string) => {
    const q = getCurrentQueue()
    if (!q) return
    const i = q.findIndex(m => m.id === messageId)
    if (i >= 0) q.splice(i, 1)
  }

  const updateQueuedMessage = (messageId: string, content: string) => {
    const msg = getCurrentQueue()?.find(m => m.id === messageId)
    if (msg) msg.content = content
  }

  const reorderQueuedMessage = (from: number, to: number) => {
    const q = getCurrentQueue()
    if (!q || from < 0 || from >= q.length || to < 0 || to >= q.length) return
    const [item] = q.splice(from, 1)
    q.splice(to, 0, item)
  }

  const sendQueuedMessageNow = async (messageId: string) => {
    const q = getCurrentQueue()
    if (!q) return
    const i = q.findIndex(m => m.id === messageId)
    if (i < 0) return
    const [msg] = q.splice(i, 1)
    await sendMessage(msg.content, msg.images)
  }

  const processQueue = async () => {
    const q = getCurrentQueue()
    if (!q?.length) return
    const [next] = q.splice(0, 1)
    await sendMessage(next.content, next.images)
  }

  // Task execution state — single source of truth
  // idle: no task running
  // pending: sendMessage called, waiting for task_created (session unknown yet)
  // running: task_created received, we know both taskId and sessionId
  type TaskExecState =
    | { status: 'idle' }
    | { status: 'pending' }
    | { status: 'running'; taskId: string; sessionId: number }

  const taskState = ref<TaskExecState>({ status: 'idle' })

  const resetTaskState = () => {
    taskState.value = { status: 'idle' }
  }

  // All external queries derived from taskState
  const isSending = computed(() => taskState.value.status !== 'idle')
  const isCurrentSessionSending = computed(() => {
    const s = taskState.value
    if (s.status === 'idle') return false
    if (s.status === 'pending') return true
    return s.sessionId === currentSession.value?.id
  })
  const isSessionRunning = (sessionId: number): boolean => {
    const s = taskState.value
    return s.status === 'running' && s.sessionId === sessionId
  }

  // Derived
  const currentWorkspacePath = computed(() => workspaceStore.currentWorkspacePath)
  const hasWorkspace = computed(() => workspaceStore.hasWorkspace)
  const currentSession = computed(() => workspaceStore.selectedSession)
  const messageList = computed(() => workspaceStore.messages.filter(m => !m.isInternal))
  const canSendMessage = computed(() => !isSending.value && aiSettingsStore.hasModels && hasWorkspace.value)

  const extractContextUsage = () => {
    const msgs = workspaceStore.messages
    for (let i = msgs.length - 1; i >= 0; i--) {
      const msg = msgs[i]
      if (msg.contextUsage) {
        contextUsage.value = msg.contextUsage
        return
      }
    }
    contextUsage.value = null
  }

  // UI operations
  const toggleSidebar = async () => {
    layoutStore.setAiSidebarVisible(!layoutStore.aiSidebarVisible)
    if (layoutStore.aiSidebarVisible && !aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  }

  const setSidebarWidth = (width: number) => {
    layoutStore.setAiSidebarWidth(width)
  }

  const setChatMode = (mode: ChatMode) => {
    layoutStore.setAiChatMode(mode)
  }

  // Session operations
  const isCreatingSession = ref(false)

  const startNewChat = async () => {
    if (isCreatingSession.value) return
    isCreatingSession.value = true
    try {
      const path = currentWorkspacePath.value
      if (path) {
        await workspaceStore.createSession(path)
      } else {
        workspaceStore.clearSelection()
      }
      contextUsage.value = null
    } finally {
      isCreatingSession.value = false
    }
  }

  const switchSession = async (sessionId: number) => {
    const path = currentWorkspacePath.value
    const node = workspaceStore.getNode(path)
    const session = node?.sessions.find(s => s.id === sessionId)
    if (session) {
      await workspaceStore.selectSession(session)
      extractContextUsage()
    }
  }

  // Agent event handling
  const handleAgentEvent = (event: TaskProgressPayload) => {
    switch (event.type) {
      case 'task_retrying':
        retryStatus.value = {
          attempt: event.attempt,
          maxAttempts: event.maxAttempts,
          reason: event.reason,
          errorMessage: event.errorMessage,
        }
        return
      case 'message_created':
        retryStatus.value = null
        workspaceStore.upsertMessage(event.message)
        break
      case 'block_appended':
        retryStatus.value = null
        workspaceStore.appendBlock(event.messageId, event.block)
        break
      case 'block_updated':
        workspaceStore.updateBlock(event.messageId, event.blockId, event.block)
        break
      case 'message_finished': {
        workspaceStore.finishMessage(event.messageId, {
          status: event.status,
          finishedAt: event.finishedAt,
          durationMs: event.durationMs,
          tokenUsage: event.tokenUsage,
          contextUsage: event.contextUsage,
        })
        if (event.contextUsage) {
          contextUsage.value = event.contextUsage
        }
        const s = taskState.value
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (s.status === 'running' && msg && msg.sessionId === s.sessionId && !msg.isSummary) {
          resetTaskState()
        }
        break
      }
      case 'tool_confirmation_requested':
        toolConfirmStore.open({
          requestId: event.requestId,
          workspacePath: event.workspacePath,
          toolName: event.toolName,
          summary: event.summary,
        })
        break
      case 'task_completed':
        retryStatus.value = null
        if (taskState.value.status !== 'running' || event.taskId === taskState.value.taskId) {
          resetTaskState()
          toolConfirmStore.close()
        }
        // Reload sessions to get updated title
        if (currentWorkspacePath.value) {
          workspaceStore.loadSessions(currentWorkspacePath.value)
        }
        break
      case 'task_cancelled':
      case 'task_error':
        retryStatus.value = null
        if (taskState.value.status !== 'running' || event.taskId === taskState.value.taskId) {
          resetTaskState()
          toolConfirmStore.close()
        }
        break
    }
  }

  const attachStreamHandlers = (stream: TaskProgressStream) => {
    let cancelSent = false
    let processingPromise: Promise<void> | null = null

    const processEvent = async (event: TaskProgressPayload) => {
      if (event.type === 'task_created') {
        taskState.value = { status: 'running', taskId: event.taskId, sessionId: event.sessionId }
        await workspaceStore.selectSessionById(event.sessionId, event.workspacePath)
        return
      }

      if (event.type === 'message_created' && event.message.isSummary) {
        await workspaceStore.fetchMessages(event.message.sessionId)
        return
      }

      if (!cancelSent && cancelRequested.value && taskState.value.status === 'running') {
        cancelSent = true
        await agentApi.cancelTask(taskState.value.taskId)
      }

      handleAgentEvent(event)
    }

    // Serialize event processing to avoid race conditions
    stream.onProgress(event => {
      const work = async () => {
        if (processingPromise) await processingPromise
        await processEvent(event)
      }
      processingPromise = work()
    })

    stream.onError((streamError: Error) => {
      console.error('Agent task error:', streamError)
      resetTaskState()
    })

    stream.onClose(() => {
      cancelFunction.value = null
      cancelRequested.value = false
      resetTaskState()
      retryStatus.value = null
      // Auto-send next queued message only on natural completion
      if (!userCancelled.value) {
        void processQueue()
      }
      userCancelled.value = false
    })

    cancelFunction.value = () => {
      if (cancelRequested.value) return
      cancelRequested.value = true
      toolConfirmStore.close()
      const s = taskState.value
      if (s.status === 'running' && !cancelSent) {
        cancelSent = true
        void agentApi.cancelTask(s.taskId)
      }
      resetTaskState()
    }
  }

  // Send message
  const sendMessage = async (content: string, images?: ImageAttachment[]): Promise<void> => {
    const workspacePath = currentWorkspacePath.value
    if (!workspacePath) {
      error.value = 'Please open a workspace folder first'
      return
    }

    if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }

    const parseAgentOverride = (text: string): { agentType?: string; prompt: string } => {
      const trimmed = text.trim()
      if (!trimmed) return { prompt: text }
      const lower = trimmed.toLowerCase()
      if (lower.startsWith('/explore ') || lower === '/explore') {
        return { agentType: 'explore', prompt: trimmed.replace(/^\/explore\b\s*/i, '') }
      }
      if (trimmed.startsWith('用explore') || trimmed.startsWith('使用explore')) {
        return { agentType: 'explore', prompt: trimmed.replace(/^(用|使用)explore\s*/i, '') }
      }
      return { prompt: text }
    }

    const selectedModelId = layoutStore.selectedModelId || aiSettingsStore.chatModels[0]?.id
    if (!selectedModelId) {
      throw new Error('Please select a model in settings first')
    }

    const sessionId = currentSession.value?.id ?? 0

    taskState.value = { status: 'pending' }
    error.value = null

    const { agentType, prompt } = parseAgentOverride(content)

    // Consume pending command ID (set by ChatInput, cleared after use)
    const commandId = pendingCommandId.value ?? undefined
    pendingCommandId.value = null

    const stream = await agentApi.executeTask({
      workspacePath,
      sessionId,
      userPrompt: prompt,
      modelId: selectedModelId,
      agentType,
      commandId,
      images: images?.map(img => ({
        type: 'image' as const,
        dataUrl: img.dataUrl,
        mimeType: img.mimeType,
      })),
    })

    if (!stream) {
      resetTaskState()
      throw new Error('Failed to create task stream')
    }

    attachStreamHandlers(stream)
  }

  const stopCurrentTask = (): void => {
    if (isSending.value && cancelFunction.value) {
      userCancelled.value = true
      try {
        cancelFunction.value()
      } catch (e) {
        console.warn('Failed to stop task:', e)
      } finally {
        cancelFunction.value = null
        resetTaskState()
      }
    }
  }

  const clearError = (): void => {
    error.value = null
  }

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await layoutStore.initialize()

    // Load workspace tree
    await workspaceStore.loadTree()
    extractContextUsage()

    isInitialized.value = true
  }

  return {
    // UI state
    isVisible,
    sidebarWidth,
    chatMode,
    isInitialized,
    error,
    canSendMessage,
    contextUsage,
    retryStatus,

    // Task state (derived from single source of truth)
    isSending,
    isCurrentSessionSending,
    isSessionRunning,

    // Derived
    messageList,
    currentSession,
    currentWorkspacePath,
    hasWorkspace,

    // Operations
    toggleSidebar,
    setSidebarWidth,
    setChatMode,
    startNewChat,
    switchSession,
    sendMessage,
    stopCurrentTask,
    clearError,
    initialize,
    pendingCommandId,

    // Message queue
    currentSessionQueue,
    enqueueMessage,
    removeQueuedMessage,
    updateQueuedMessage,
    reorderQueuedMessage,
    sendQueuedMessageNow,
  }
})
