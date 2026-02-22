<script setup lang="ts">
  import { workspaceApi, type SessionRecord, type WorkspaceRecord } from '@/api/workspace'
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import { useLayoutStore } from '@/stores/layout'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { confirmDanger } from '@/ui'
  import { formatRelativeTime } from '@/utils/dateFormatter'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { open } from '@tauri-apps/plugin-dialog'
  import { storeToRefs } from 'pinia'
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()
  const layoutStore = useLayoutStore()
  const workspaceStore = useWorkspaceStore()
  const aiChatStore = useAIChatStore()

  const { showSettings } = storeToRefs(layoutStore)
  const { selectedSession } = storeToRefs(workspaceStore)

  const props = defineProps<{
    isVisible: boolean
    width: number
    showSkills: boolean
  }>()

  const emit = defineEmits<{
    (e: 'update:width', width: number): void
    (e: 'update:showSkills', show: boolean): void
    (e: 'toggle-sidebar'): void
    (e: 'drag-start'): void
    (e: 'drag-end'): void
  }>()

  // ============ Sidebar Resize ============
  const isDragging = ref(false)

  const startResize = (event: MouseEvent) => {
    event.preventDefault()
    isDragging.value = true
    emit('drag-start')

    const startX = event.clientX
    const startWidth = props.width

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaX = e.clientX - startX
      const newWidth = Math.max(180, Math.min(startWidth + deltaX, 400))
      emit('update:width', newWidth)
    }

    const handleMouseUp = () => {
      isDragging.value = false
      emit('drag-end')
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  // ============ Window Actions ============
  const startWindowDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  const handleDoubleClick = async () => {
    const win = getCurrentWindow()
    const isMaximized = await win.isMaximized()
    if (isMaximized) {
      await win.unmaximize()
    } else {
      await win.maximize()
    }
  }

  // ============ Settings Navigation ============
  const activeSettingsSection = ref('general')
  const settingsNavItems = computed(() => [
    { id: 'general', label: t('settings.general.title') },
    { id: 'ai', label: t('settings.ai.title') },
    { id: 'mcp', label: t('mcp_settings.title') },
    { id: 'theme', label: t('settings.theme.title') },
    { id: 'shortcuts', label: t('settings.shortcuts.title') },
    { id: 'language', label: t('settings.language.title') },
  ])

  const handleSettingsNavChange = (section: string) => {
    activeSettingsSection.value = section
  }

  const handleBackFromSettings = () => {
    layoutStore.closeSettings()
  }

  // ============ Skills Actions ============
  const handleOpenSkills = () => {
    emit('update:showSkills', true)
  }

  const expandedPaths = ref<Set<string>>(new Set())
  const currentSessionId = computed(() => selectedSession.value?.id ?? null)

  // Auto-expand workspace folder when a session is selected (e.g. on restore)
  watch(
    selectedSession,
    session => {
      if (session && !expandedPaths.value.has(session.workspacePath)) {
        expandedPaths.value.add(session.workspacePath)
        expandedPaths.value = new Set(expandedPaths.value)
      }
    },
    { immediate: true }
  )

  const getWorkspaceName = (workspace: WorkspaceRecord) => {
    if (workspace.displayName) return workspace.displayName
    return workspace.path.split('/').pop() || workspace.path
  }

  const isExpanded = (path: string) => expandedPaths.value.has(path)
  const getNode = (path: string) => workspaceStore.getNode(path)

  const handleToggleWorkspace = async (path: string) => {
    workspaceStore.setActiveWorkspace(path)
    if (expandedPaths.value.has(path)) {
      expandedPaths.value.delete(path)
    } else {
      expandedPaths.value.add(path)
      const node = workspaceStore.getNode(path)
      if (node && node.sessions.length === 0 && !node.isLoading) {
        await workspaceStore.loadSessions(path)
      }
    }
    expandedPaths.value = new Set(expandedPaths.value)
  }

  const isWorkspaceActive = (path: string) => workspaceStore.activeWorkspacePath === path

  const handleSelectSession = async (session: SessionRecord) => {
    emit('update:showSkills', false)
    await workspaceStore.selectSession(session)
  }

  const handleNewThread = async () => {
    emit('update:showSkills', false)
    await aiChatStore.startNewChat()
  }

  const handleOpenFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Workspace Folder',
    })
    if (selected && typeof selected === 'string') {
      // First, ensure the workspace is created/registered in the backend
      await workspaceApi.getOrCreate(selected)
      // Now reload the tree to include the new workspace
      await workspaceStore.loadTree()
      // Set as active workspace
      workspaceStore.setActiveWorkspace(selected)
      // Expand and load sessions
      expandedPaths.value.add(selected)
      expandedPaths.value = new Set(expandedPaths.value)
      await workspaceStore.loadSessions(selected)
    }
  }

  const handleOpenSettings = () => {
    layoutStore.openSettings()
  }

  const getSessionTitle = (session: { title?: string | null; id: number }) => {
    return session.title || t('sidebar.new_thread')
  }

  const isSessionActive = (session: SessionRecord) => session.id === currentSessionId.value
  const isSessionLoading = (session: SessionRecord) => aiChatStore.isSessionRunning(session.id)

  const handleNewSessionInWorkspace = async (event: MouseEvent, workspacePath: string) => {
    event.stopPropagation()
    emit('update:showSkills', false)
    workspaceStore.setActiveWorkspace(workspacePath)
    // Expand the workspace folder
    if (!expandedPaths.value.has(workspacePath)) {
      expandedPaths.value.add(workspacePath)
      expandedPaths.value = new Set(expandedPaths.value)
    }
    await workspaceStore.createSession(workspacePath)
  }

  const handleDeleteWorkspace = async (event: MouseEvent, workspacePath: string) => {
    event.stopPropagation()
    const confirmed = await confirmDanger(
      t('sidebar.delete_workspace_confirm') || 'Are you sure you want to delete this workspace?',
      t('sidebar.delete_workspace') || 'Delete Workspace'
    )
    if (!confirmed) return
    await workspaceStore.deleteWorkspace(workspacePath)
  }

  const handleDeleteSession = async (event: MouseEvent, session: SessionRecord) => {
    event.stopPropagation()
    const confirmed = await confirmDanger(
      t('sidebar.delete_session_confirm') || 'Are you sure you want to delete this session?',
      t('sidebar.delete_session') || 'Delete Session'
    )
    if (!confirmed) return
    await workspaceStore.deleteSession(session.id, session.workspacePath)
  }

  defineExpose({ activeSettingsSection })
</script>

<template>
  <aside
    class="sidebar"
    :class="{ 'sidebar--collapsed': !isVisible && !showSettings }"
    :style="{ width: isVisible || showSettings ? `${width}px` : '0' }"
  >
    <div class="sidebar-inner" :style="{ width: `${width}px` }">
      <!-- Settings Sidebar -->
      <template v-if="showSettings">
        <div class="sidebar-header" @mousedown="startWindowDrag" @dblclick="handleDoubleClick" />
        <button class="back-btn" @click="handleBackFromSettings">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M15 19l-7-7 7-7" />
          </svg>
          {{ t('settings.back_to_app') }}
        </button>
        <nav class="settings-nav">
          <button
            v-for="item in settingsNavItems"
            :key="item.id"
            class="nav-item"
            :class="{ active: activeSettingsSection === item.id }"
            @click="handleSettingsNavChange(item.id)"
          >
            {{ item.label }}
          </button>
        </nav>
      </template>

      <!-- Threads Sidebar -->
      <template v-else>
        <div class="sidebar-header" @mousedown="startWindowDrag" @dblclick="handleDoubleClick" />
        <button class="new-thread-btn" @click="handleNewThread">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M12 5l0 14" />
            <path d="M5 12l14 0" />
          </svg>
          <span>{{ t('sidebar.new_thread') }}</span>
        </button>

        <button class="new-thread-btn skills-btn" :class="{ active: showSkills }" @click="handleOpenSkills">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a2.5 2.5 0 0 1 0-5H20" />
          </svg>
          <span>{{ t('sidebar.skills') }}</span>
        </button>

        <div class="threads-section">
          <div class="section-header">
            <span class="section-title">{{ t('sidebar.threads') }}</span>
            <button class="icon-btn" :title="t('sidebar.open_folder')" @click="handleOpenFolder">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M12 5v14m7-7H5" />
              </svg>
            </button>
          </div>

          <div class="threads-list">
            <div v-for="workspace in workspaceStore.workspaces" :key="workspace.path" class="workspace-group">
              <div
                class="workspace-item"
                :class="{ expanded: isExpanded(workspace.path), active: isWorkspaceActive(workspace.path) }"
                @click="handleToggleWorkspace(workspace.path)"
              >
                <span class="workspace-icon-slot" :class="{ expanded: isExpanded(workspace.path) }">
                  <!-- Arrow (shown on hover or when expanded) -->
                  <svg
                    class="expand-icon"
                    :class="{ expanded: isExpanded(workspace.path) }"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M6 4l4 4-4 4" />
                  </svg>
                  <!-- Folder open -->
                  <svg
                    v-if="isExpanded(workspace.path)"
                    class="folder-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path
                      d="M6 14l1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"
                    />
                  </svg>
                  <!-- Folder closed -->
                  <svg
                    v-else
                    class="folder-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path
                      d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h16z"
                    />
                  </svg>
                </span>
                <span class="workspace-name">{{ getWorkspaceName(workspace) }}</span>
                <span v-if="getNode(workspace.path)?.isLoading" class="loading-indicator">...</span>
                <span class="workspace-actions">
                  <button
                    class="action-btn add-btn"
                    :title="t('chat.new_session')"
                    @click="handleNewSessionInWorkspace($event, workspace.path)"
                  >
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M12 5v14" />
                      <path d="M5 12h14" />
                    </svg>
                  </button>
                  <button
                    class="action-btn delete-btn"
                    :title="t('sidebar.delete_workspace')"
                    @click="handleDeleteWorkspace($event, workspace.path)"
                  >
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M3 6h18" />
                      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                    </svg>
                  </button>
                </span>
              </div>

              <Transition name="tree-expand">
                <div v-if="isExpanded(workspace.path)" class="sessions-list">
                  <div v-if="!getNode(workspace.path)?.sessions.length" class="no-sessions">
                    {{ t('sidebar.no_threads') }}
                  </div>
                  <div
                    v-for="session in getNode(workspace.path)?.sessions || []"
                    :key="session.id"
                    class="session-item"
                    :class="{ active: isSessionActive(session), loading: isSessionLoading(session) }"
                    @click.stop="handleSelectSession(session)"
                  >
                    <span v-if="isSessionLoading(session)" class="session-loading">
                      <svg viewBox="0 0 16 16" fill="none">
                        <path d="M8 2a6 6 0 0 1 0 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                      </svg>
                    </span>
                    <span class="session-title">{{ getSessionTitle(session) }}</span>
                    <span class="session-trailing">
                      <span class="session-time">{{ formatRelativeTime(session.updatedAt * 1000) }}</span>
                      <button
                        class="delete-btn"
                        :title="t('sidebar.delete_session')"
                        @click="handleDeleteSession($event, session)"
                      >
                        <svg
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="1.5"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                        >
                          <path d="M3 6h18" />
                          <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                          <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                        </svg>
                      </button>
                    </span>
                  </div>
                </div>
              </Transition>
            </div>

            <div v-if="workspaceStore.workspaces.length === 0" class="empty-state">
              <span>{{ t('sidebar.no_workspaces') }}</span>
            </div>
          </div>
        </div>

        <div class="sidebar-footer">
          <button class="settings-btn" @click="handleOpenSettings">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
              />
              <circle cx="12" cy="12" r="3" />
            </svg>
            <span>{{ t('settings.title') }}</span>
          </button>
        </div>
      </template>
    </div>

    <!-- Sidebar Resize Handle -->
    <div class="sidebar-resize-handle" :class="{ active: isDragging }" @mousedown="startResize" />
  </aside>
</template>

<style scoped>
  /* ========== SIDEBAR ========== */
  .sidebar {
    display: flex;
    flex-direction: column;
    background: var(--sidebar-glass-bg);
    transition:
      width 0.25s ease,
      background 0.25s ease;
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  .sidebar--collapsed {
    width: 0 !important;
  }

  .sidebar-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    flex-shrink: 0;
  }

  .sidebar-resize-handle {
    position: absolute;
    top: 0;
    right: -4px;
    width: 12px;
    height: 100%;
    background: transparent;
    cursor: col-resize;
    z-index: 10;
  }

  .sidebar-resize-handle::after {
    content: '';
    position: absolute;
    top: 12px;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    width: 1px;
    background: transparent;
    transition: background 0.15s ease;
    pointer-events: none;
  }

  .sidebar-resize-handle:hover::after,
  .sidebar-resize-handle.active::after {
    background: var(--color-primary);
  }

  .sidebar-header {
    height: 40px;
    flex-shrink: 0;
    -webkit-app-region: drag;
  }

  /* Back button for settings */
  .back-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    cursor: pointer;
  }

  .back-btn:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .back-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Settings nav */
  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
  }

  .nav-item {
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    cursor: pointer;
    text-align: left;
  }

  .nav-item:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .nav-item.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  /* Threads sidebar */
  .new-thread-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-300);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    width: calc(100% - 16px);
  }

  .new-thread-btn:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .new-thread-btn svg {
    width: 18px;
    height: 18px;
    opacity: 0.7;
    flex-shrink: 0;
  }

  /* Skills button active state */
  .skills-btn.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  .skills-btn.active svg {
    color: var(--text-300);
    opacity: 0.7;
  }

  .threads-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    margin-top: 8px;
    overflow: hidden;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px 6px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-500);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .icon-btn {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-500);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--color-hover);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }

  .threads-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  .threads-list::-webkit-scrollbar {
    width: 4px;
  }

  .threads-list::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: var(--border-radius-xs);
  }

  .workspace-group {
    margin-bottom: 4px;
  }

  .workspace-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-300);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .workspace-item:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .workspace-item.expanded {
    color: var(--text-200);
  }

  .workspace-item.active {
    color: var(--text-100);
  }

  .workspace-item.active .folder-icon {
    opacity: 0.9;
  }

  .workspace-icon-slot {
    position: relative;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .workspace-icon-slot .expand-icon,
  .workspace-icon-slot .folder-icon {
    position: absolute;
  }

  /* Default: show folder, hide arrow */
  .workspace-icon-slot .expand-icon {
    opacity: 0;
  }

  .workspace-icon-slot .folder-icon {
    opacity: 0.6;
  }

  /* Hover: show arrow, hide folder */
  .workspace-item:hover .workspace-icon-slot .expand-icon {
    opacity: 0.5;
  }

  .workspace-item:hover .workspace-icon-slot .folder-icon {
    opacity: 0;
  }

  /* Expanded + hover: show arrow, hide folder */
  .workspace-item:hover .workspace-icon-slot.expanded .expand-icon {
    opacity: 0.7;
  }

  .workspace-item:hover .workspace-icon-slot.expanded .folder-icon {
    opacity: 0;
  }

  .expand-icon {
    width: 14px;
    height: 14px;
    transition: transform 0.15s ease;
  }

  .expand-icon.expanded {
    transform: rotate(90deg);
  }

  .folder-icon {
    width: 16px;
    height: 16px;
  }

  .workspace-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* Workspace action buttons container */
  .workspace-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .workspace-item:hover .workspace-actions {
    opacity: 1;
  }

  /* Shared action button style (add + delete) */
  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-500);
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease;
    flex-shrink: 0;
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .action-btn.delete-btn:hover {
    color: var(--color-error, #ef4444);
  }

  /* Session-level delete button */
  .session-trailing .delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    position: absolute;
    right: 0;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-500);
    cursor: pointer;
    opacity: 0;
    transition:
      opacity 0.15s ease,
      background 0.15s ease,
      color 0.15s ease;
    flex-shrink: 0;
  }

  .session-trailing .delete-btn svg {
    width: 14px;
    height: 14px;
  }

  .session-trailing .delete-btn:hover {
    background: var(--color-hover);
    color: var(--color-error, #ef4444);
  }

  .loading-indicator {
    font-size: 12px;
    color: var(--text-500);
    flex-shrink: 0;
  }

  .no-sessions {
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text-500);
  }

  .sessions-list {
    padding-left: 20px;
    padding-top: 4px;
    padding-bottom: 4px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* Tree expand/collapse animation */
  .tree-expand-enter-active,
  .tree-expand-leave-active {
    transition:
      max-height 0.2s ease,
      opacity 0.2s ease;
    max-height: 500px;
  }

  .tree-expand-enter-from,
  .tree-expand-leave-to {
    max-height: 0;
    opacity: 0;
  }

  .session-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-300);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .session-item:hover {
    background: var(--color-hover);
  }

  .session-item.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  .session-item.loading {
    color: var(--text-200);
  }

  /* Loading indicator */
  .session-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .session-loading svg {
    width: 12px;
    height: 12px;
    color: var(--text-400);
    animation: spin-loading 1s linear infinite;
  }

  @keyframes spin-loading {
    to {
      transform: rotate(360deg);
    }
  }

  .session-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-trailing {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .session-item:hover .session-trailing .session-time {
    opacity: 0;
  }

  .session-item:hover .session-trailing .delete-btn {
    opacity: 1;
  }

  .session-time {
    font-size: 12px;
    color: var(--text-500);
    white-space: nowrap;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    font-size: 13px;
    color: var(--text-500);
  }

  .sidebar-footer {
    padding: 8px 12px;
    margin-top: auto;
  }

  .settings-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-300);
    font-size: 14px;
    cursor: pointer;
  }

  .settings-btn:hover {
    background: var(--color-hover);
  }

  .settings-btn svg {
    width: 18px;
    height: 18px;
    opacity: 0.7;
  }
</style>
