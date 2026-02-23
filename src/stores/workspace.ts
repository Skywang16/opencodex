import { workspaceApi, type SessionRecord, type WorkspaceRecord } from '@/api/workspace'
import type { Message } from '@/types'
import { defineStore } from 'pinia'
import { computed, reactive, ref, shallowRef } from 'vue'

const MAX_CACHED_SESSIONS = 10

export interface WorkspaceNode {
  workspace: WorkspaceRecord
  sessions: SessionRecord[]
  isLoading: boolean
}

export const useWorkspaceStore = defineStore('workspace', () => {
  // State
  const tree = shallowRef<Map<string, WorkspaceNode>>(new Map())
  const selectedSession = ref<SessionRecord | null>(null)
  const messagesBySessionId = reactive<Map<number, Message[]>>(new Map())
  const messageIdToSessionId = new Map<number, number>()
  const activeWorkspacePath = ref<string | null>(null)
  const sessionAccessOrder: number[] = [] // LRU tracking: oldest first

  // Computed
  const messages = computed<Message[]>(() => {
    const sessionId = selectedSession.value?.id
    if (!sessionId) return []
    return messagesBySessionId.get(sessionId) ?? []
  })

  const workspaces = computed(() => {
    return Array.from(tree.value.values())
      .map(node => node.workspace)
      .sort((a, b) => b.lastAccessedAt - a.lastAccessedAt)
  })

  const currentWorkspacePath = computed(() => activeWorkspacePath.value)

  const hasWorkspace = computed(() => currentWorkspacePath.value !== null)

  // LRU cache management
  const touchSession = (sessionId: number) => {
    const idx = sessionAccessOrder.indexOf(sessionId)
    if (idx >= 0) sessionAccessOrder.splice(idx, 1)
    sessionAccessOrder.push(sessionId)
  }

  const evictOldSessions = () => {
    const currentId = selectedSession.value?.id
    let guard = sessionAccessOrder.length
    while (sessionAccessOrder.length > MAX_CACHED_SESSIONS && guard > 0) {
      guard--
      const oldest = sessionAccessOrder[0]
      if (oldest === currentId) {
        sessionAccessOrder.shift()
        sessionAccessOrder.push(oldest)
        continue
      }
      sessionAccessOrder.shift()
      const cached = messagesBySessionId.get(oldest)
      if (cached) {
        for (const msg of cached) messageIdToSessionId.delete(msg.id)
      }
      messagesBySessionId.delete(oldest)
    }
  }

  // Internal helpers
  const indexMessageList = (list: Message[]) => {
    for (const msg of list) {
      messageIdToSessionId.set(msg.id, msg.sessionId)
    }
  }

  const setSessionMessages = (sessionId: number, list: Message[]) => {
    messagesBySessionId.set(sessionId, reactive(list))
    indexMessageList(list)
    touchSession(sessionId)
    evictOldSessions()
  }

  const ensureSessionMessages = (sessionId: number): Message[] => {
    touchSession(sessionId)
    const existing = messagesBySessionId.get(sessionId)
    if (existing) return existing
    const created = reactive<Message[]>([])
    messagesBySessionId.set(sessionId, created)
    evictOldSessions()
    return created
  }

  const resolveSessionIdByMessageId = (messageId: number): number | null => {
    const indexed = messageIdToSessionId.get(messageId)
    if (typeof indexed === 'number') return indexed
    for (const [sessionId, list] of messagesBySessionId.entries()) {
      if (list.some(m => m.id === messageId)) {
        messageIdToSessionId.set(messageId, sessionId)
        return sessionId
      }
    }
    return null
  }

  // Tree operations
  const loadTree = async () => {
    const list = await workspaceApi.listRecent(20)
    const newTree = new Map<string, WorkspaceNode>()
    for (const ws of list) {
      newTree.set(ws.path, {
        workspace: ws,
        sessions: tree.value.get(ws.path)?.sessions || [],
        isLoading: false,
      })
    }
    tree.value = newTree

    // Restore last active session
    if (!selectedSession.value) {
      for (const ws of list) {
        if (ws.activeSessionId) {
          await selectSessionById(ws.activeSessionId, ws.path)
          break
        }
      }
    }
  }

  const loadSessions = async (path: string) => {
    const node = tree.value.get(path)
    if (!node || node.isLoading) return

    // Mark as loading
    tree.value = new Map(tree.value).set(path, { ...node, isLoading: true })

    try {
      const sessions = await workspaceApi.listSessions(path)
      tree.value = new Map(tree.value).set(path, {
        ...node,
        sessions,
        isLoading: false,
      })
    } catch {
      tree.value = new Map(tree.value).set(path, { ...node, isLoading: false })
    }
  }

  const getNode = (path: string | null): WorkspaceNode | undefined => {
    if (!path) return undefined
    return tree.value.get(path)
  }

  // Session operations
  const selectSession = async (session: SessionRecord) => {
    if (selectedSession.value?.id === session.id) return
    selectedSession.value = session
    activeWorkspacePath.value = session.workspacePath
    workspaceApi.setActiveSession(session.workspacePath, session.id)
    setSessionMessages(session.id, [])
    const loaded = await workspaceApi.getMessages(session.id)
    setSessionMessages(session.id, loaded)
  }

  const selectSessionById = async (sessionId: number, workspacePath: string) => {
    if (selectedSession.value?.id === sessionId) return
    const sessions = await workspaceApi.listSessions(workspacePath)
    const existingNode = tree.value.get(workspacePath)
    const workspace: WorkspaceRecord = existingNode?.workspace ?? {
      path: workspacePath,
      displayName: null,
      lastAccessedAt: Date.now(),
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }
    tree.value = new Map(tree.value).set(workspacePath, {
      workspace,
      sessions,
      isLoading: false,
    })
    const session = sessions.find(s => s.id === sessionId)
    if (session) {
      selectedSession.value = session
      activeWorkspacePath.value = workspacePath
      setSessionMessages(sessionId, [])
      const loaded = await workspaceApi.getMessages(sessionId)
      setSessionMessages(sessionId, loaded)
    }
  }

  const clearSelection = () => {
    selectedSession.value = null
  }

  const setActiveWorkspace = (path: string | null) => {
    activeWorkspacePath.value = path
  }

  const deleteSession = async (sessionId: number, workspacePath: string) => {
    await workspaceApi.deleteSession(sessionId)
    const node = tree.value.get(workspacePath)
    if (node) {
      tree.value = new Map(tree.value).set(workspacePath, {
        ...node,
        sessions: node.sessions.filter(s => s.id !== sessionId),
      })
    }
    if (selectedSession.value?.id === sessionId) {
      selectedSession.value = null
      workspaceApi.clearActiveSession(workspacePath)
    }
    const cached = messagesBySessionId.get(sessionId)
    if (cached) {
      for (const msg of cached) messageIdToSessionId.delete(msg.id)
    }
    messagesBySessionId.delete(sessionId)
  }

  const deleteWorkspace = async (workspacePath: string) => {
    await workspaceApi.deleteWorkspace(workspacePath)
    const node = tree.value.get(workspacePath)
    if (node) {
      for (const session of node.sessions) {
        const cached = messagesBySessionId.get(session.id)
        if (cached) {
          for (const msg of cached) messageIdToSessionId.delete(msg.id)
        }
        messagesBySessionId.delete(session.id)
      }
    }
    const newTree = new Map(tree.value)
    newTree.delete(workspacePath)
    tree.value = newTree
    if (selectedSession.value?.workspacePath === workspacePath) {
      selectedSession.value = null
    }
    if (activeWorkspacePath.value === workspacePath) {
      activeWorkspacePath.value = null
    }
  }

  const createSession = async (workspacePath: string, title?: string) => {
    await workspaceApi.getOrCreate(workspacePath)
    const session = await workspaceApi.createSession(workspacePath, title ?? 'New Chat')
    const node = tree.value.get(workspacePath)
    if (node) {
      tree.value = new Map(tree.value).set(workspacePath, {
        ...node,
        sessions: [session, ...node.sessions],
      })
    }
    selectedSession.value = session
    activeWorkspacePath.value = workspacePath
    workspaceApi.setActiveSession(workspacePath, session.id)
    setSessionMessages(session.id, [])
    return session
  }

  // Message operations (for stream updates)
  const upsertMessage = (message: Message) => {
    const list = ensureSessionMessages(message.sessionId)
    const idx = list.findIndex(m => m.id === message.id)
    if (idx >= 0) {
      list[idx] = message
    } else {
      list.push(message)
    }
    messageIdToSessionId.set(message.id, message.sessionId)
  }

  const appendBlock = (messageId: number, block: Message['blocks'][number]) => {
    const sessionId = resolveSessionIdByMessageId(messageId)
    if (!sessionId) {
      console.warn(`[workspace] appendBlock: session not found for message ${messageId}`)
      return
    }
    const list = messagesBySessionId.get(sessionId)
    if (!list) {
      console.warn(`[workspace] appendBlock: message list not found for session ${sessionId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] appendBlock: message ${messageId} not found`)
      return
    }
    msg.blocks.push(block)
  }

  const updateBlock = (messageId: number, blockId: string, block: Message['blocks'][number]) => {
    const sessionId = resolveSessionIdByMessageId(messageId)
    if (!sessionId) {
      console.warn(`[workspace] updateBlock: session not found for message ${messageId}`)
      return
    }
    const list = messagesBySessionId.get(sessionId)
    if (!list) {
      console.warn(`[workspace] updateBlock: message list not found for session ${sessionId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] updateBlock: message ${messageId} not found`)
      return
    }
    const idx = msg.blocks.findIndex(b => 'id' in b && b.id === blockId)
    if (idx >= 0) {
      msg.blocks[idx] = block
    } else {
      console.warn(`[workspace] updateBlock: block ${blockId} not found in message ${messageId}`)
    }
  }

  const finishMessage = (
    messageId: number,
    patch: Partial<Pick<Message, 'status' | 'finishedAt' | 'durationMs' | 'tokenUsage' | 'contextUsage'>>
  ) => {
    const sessionId = resolveSessionIdByMessageId(messageId)
    if (!sessionId) {
      console.warn(`[workspace] finishMessage: session not found for message ${messageId}`)
      return
    }
    const list = messagesBySessionId.get(sessionId)
    if (!list) {
      console.warn(`[workspace] finishMessage: message list not found for session ${sessionId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] finishMessage: message ${messageId} not found`)
      return
    }
    Object.assign(msg, patch)
  }

  const fetchMessages = async (sessionId: number) => {
    const loaded = await workspaceApi.getMessages(sessionId)
    setSessionMessages(sessionId, loaded)
  }

  const getCachedMessages = (sessionId: number) => messagesBySessionId.get(sessionId) ?? []

  return {
    // State
    tree,
    selectedSession,
    messages,
    activeWorkspacePath,
    workspaces,
    currentWorkspacePath,
    hasWorkspace,
    // Tree
    loadTree,
    loadSessions,
    getNode,
    // Session
    selectSession,
    selectSessionById,
    clearSelection,
    setActiveWorkspace,
    createSession,
    deleteSession,
    deleteWorkspace,
    // Message
    upsertMessage,
    appendBlock,
    updateBlock,
    finishMessage,
    fetchMessages,
    getCachedMessages,
  }
})
