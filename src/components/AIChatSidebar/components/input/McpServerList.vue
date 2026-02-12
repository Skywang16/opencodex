<script setup lang="ts">
  import { mcpApi, type McpServerStatus } from '@/api'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Emits {
    (e: 'back'): void
  }

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const workspaceStore = useWorkspaceStore()
  const servers = ref<McpServerStatus[]>([])
  const loading = ref(false)

  const currentWorkspace = computed(() => workspaceStore.currentWorkspacePath)

  const loadServers = async () => {
    if (!currentWorkspace.value) return
    loading.value = true
    try {
      servers.value = await mcpApi.listServers(currentWorkspace.value)
    } finally {
      loading.value = false
    }
  }

  const getStatusColor = (status: McpServerStatus['status']) => {
    switch (status) {
      case 'connected':
        return '#52c41a'
      case 'error':
        return '#ff4d4f'
      default:
        return '#8c8c8c'
    }
  }

  const getStatusText = (status: McpServerStatus['status']) => {
    switch (status) {
      case 'connected':
        return t('mcp_dialog.status_connected')
      case 'error':
        return t('mcp_dialog.status_error')
      default:
        return t('mcp_dialog.status_disconnected')
    }
  }

  onMounted(() => {
    loadServers()
  })
</script>

<template>
  <div class="mcp-list">
    <div class="mcp-header">
      <button class="back-btn" @click="emit('back')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </button>
      <span class="header-title">{{ t('mcp_dialog.title') }}</span>
    </div>

    <div v-if="!currentWorkspace" class="empty-state">
      <p>{{ t('mcp_dialog.no_workspace') }}</p>
    </div>

    <div v-else-if="loading" class="loading-state">
      <p>{{ t('mcp_dialog.loading') }}</p>
    </div>

    <div v-else-if="servers.length === 0" class="empty-state">
      <p>{{ t('mcp_dialog.no_servers') }}</p>
    </div>

    <div v-else class="servers-list">
      <div v-for="server in servers" :key="server.name" class="server-item">
        <div class="server-header">
          <div class="server-name">{{ server.name }}</div>
          <div class="server-status" :style="{ color: getStatusColor(server.status) }">
            <span class="status-dot" :style="{ backgroundColor: getStatusColor(server.status) }" />
            {{ getStatusText(server.status) }}
          </div>
        </div>

        <div class="server-meta">
          <span class="server-source">
            {{ server.source === 'workspace' ? t('mcp_dialog.workspace') : t('mcp_dialog.global') }}
          </span>
          <span v-if="server.tools.length > 0" class="server-tools">
            {{ t('mcp_dialog.tools_count', { count: server.tools.length }) }}
          </span>
        </div>

        <div v-if="server.error" class="server-error">
          {{ server.error }}
        </div>

        <div v-if="server.tools.length > 0" class="tools-list">
          <div v-for="tool in server.tools" :key="tool.name" class="tool-item">
            <span class="tool-name">{{ tool.name }}</span>
            <span v-if="tool.description" class="tool-description">{{ tool.description }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .mcp-list {
    display: flex;
    flex-direction: column;
    max-height: 400px;
  }

  .mcp-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-color);
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: color 0.15s;
  }

  .back-btn:hover {
    color: var(--text-primary);
  }

  .back-btn svg {
    width: 16px;
    height: 16px;
  }

  .header-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-state,
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 120px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .servers-list {
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .server-item {
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }

  .server-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .server-name {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
  }

  .server-status {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
  }

  .status-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
  }

  .server-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .server-source {
    padding: 1px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
  }

  .server-error {
    margin-top: 6px;
    padding: 6px;
    background: rgba(255, 77, 79, 0.1);
    border-radius: 4px;
    font-size: 11px;
    color: #ff4d4f;
  }

  .tools-list {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tool-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    background: var(--bg-primary);
    border-radius: 3px;
  }

  .tool-name {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }

  .tool-description {
    font-size: 10px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
</style>
