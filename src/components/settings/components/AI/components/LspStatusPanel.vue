<script setup lang="ts">
  import { lspApi } from '@/api'
  import type { LspServerStatus } from '@/api/lsp/types'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { XButton } from '@/ui'
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()

  const isLoading = ref(false)
  const statuses = ref<LspServerStatus[]>([])
  const error = ref<string | null>(null)

  const currentWorkspace = computed(() => workspaceStore.currentWorkspacePath)
  const activeStatuses = computed(() => {
    if (!currentWorkspace.value) return statuses.value
    return statuses.value.filter(item => item.root === currentWorkspace.value)
  })

  const load = async () => {
    isLoading.value = true
    error.value = null
    try {
      statuses.value = await lspApi.status()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  const readyCount = computed(() => activeStatuses.value.filter(item => item.connected && item.initialized).length)

  const statusLabel = (item: LspServerStatus) => {
    if (!item.connected) return t('lsp.status.offline')
    return item.initialized ? t('lsp.status.ready') : t('lsp.status.starting')
  }

  onMounted(load)
  watch(currentWorkspace, () => {
    load()
  })
</script>

<template>
  <section class="lsp-panel">
    <div class="lsp-panel__header">
      <div>
        <h2 class="lsp-panel__title">{{ t('lsp.title') }}</h2>
        <p class="lsp-panel__description">{{ t('lsp.description') }}</p>
      </div>
      <x-button variant="ghost" size="medium" :loading="isLoading" @click="load">
        {{ t('common.refresh') }}
      </x-button>
    </div>

    <div class="lsp-summary">
      <div class="lsp-summary__metric">
        <span class="lsp-summary__value">{{ readyCount }}</span>
        <span class="lsp-summary__label">{{ t('lsp.summary.ready') }}</span>
      </div>
      <div class="lsp-summary__metric">
        <span class="lsp-summary__value">{{ activeStatuses.length }}</span>
        <span class="lsp-summary__label">{{ t('lsp.summary.detected') }}</span>
      </div>
      <div class="lsp-summary__context">
        <span class="lsp-summary__label">{{ t('lsp.summary.workspace') }}</span>
        <span class="lsp-summary__workspace">{{ currentWorkspace || t('lsp.summary.no_workspace') }}</span>
      </div>
    </div>

    <div v-if="error" class="lsp-panel__error">{{ error }}</div>

    <div v-else-if="activeStatuses.length === 0" class="lsp-empty">
      <div class="lsp-empty__title">{{ t('lsp.empty.title') }}</div>
      <div class="lsp-empty__description">{{ t('lsp.empty.description') }}</div>
    </div>

    <div v-else class="lsp-list">
      <div v-for="item in activeStatuses" :key="`${item.serverId}-${item.root}`" class="lsp-card">
        <div class="lsp-card__header">
          <div>
            <div class="lsp-card__name">{{ item.serverId }}</div>
            <div class="lsp-card__command">{{ item.command }}</div>
          </div>
          <span
            class="lsp-card__badge"
            :class="{ ready: item.connected && item.initialized, starting: item.connected && !item.initialized }"
          >
            {{ statusLabel(item) }}
          </span>
        </div>
        <div class="lsp-card__grid">
          <div class="lsp-card__cell">
            <span class="lsp-card__label">{{ t('lsp.fields.documents') }}</span>
            <span class="lsp-card__value">{{ item.openDocuments }}</span>
          </div>
          <div class="lsp-card__cell">
            <span class="lsp-card__label">{{ t('lsp.fields.diagnostics') }}</span>
            <span class="lsp-card__value">{{ item.diagnosticsFiles }}</span>
          </div>
        </div>
        <div class="lsp-card__root">{{ item.root }}</div>
        <div v-if="item.lastError" class="lsp-card__error">{{ item.lastError }}</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
  .lsp-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    background:
      radial-gradient(circle at top right, color-mix(in srgb, var(--color-primary) 10%, transparent), transparent 42%),
      var(--bg-100);
  }

  .lsp-panel__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .lsp-panel__title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
  }

  .lsp-panel__description,
  .lsp-card__command,
  .lsp-card__root,
  .lsp-summary__label,
  .lsp-empty__description {
    color: var(--text-400);
    font-size: 13px;
    line-height: 1.5;
  }

  .lsp-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 120px)) minmax(0, 1fr);
    gap: 12px;
  }

  .lsp-summary__metric,
  .lsp-summary__context {
    padding: 12px;
    border-radius: var(--border-radius-lg);
    background: color-mix(in srgb, var(--bg-100) 82%, var(--bg-200));
    border: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .lsp-summary__value {
    color: var(--text-100);
    font-size: 22px;
    font-weight: 700;
    line-height: 1;
  }

  .lsp-summary__workspace {
    color: var(--text-300);
    font-size: 12px;
    font-family: var(--font-family-mono);
    word-break: break-all;
  }

  .lsp-panel__error,
  .lsp-card__error {
    color: var(--color-error);
    font-size: 12px;
    line-height: 1.5;
  }

  .lsp-empty {
    padding: 18px;
    border-radius: var(--border-radius-lg);
    border: 1px dashed var(--border-300);
    background: color-mix(in srgb, var(--bg-100) 88%, var(--bg-200));
  }

  .lsp-empty__title,
  .lsp-card__name {
    color: var(--text-200);
    font-size: 14px;
    font-weight: 600;
  }

  .lsp-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .lsp-card {
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    background: color-mix(in srgb, var(--bg-100) 90%, var(--bg-200));
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .lsp-card__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .lsp-card__badge {
    flex-shrink: 0;
    border-radius: 999px;
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-500);
    background: var(--bg-300);
  }

  .lsp-card__badge.ready {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .lsp-card__badge.starting {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .lsp-card__grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .lsp-card__cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .lsp-card__label {
    color: var(--text-500);
    font-size: 12px;
  }

  .lsp-card__value {
    color: var(--text-200);
    font-size: 16px;
    font-weight: 600;
  }

  @media (max-width: 720px) {
    .lsp-panel__header {
      flex-direction: column;
      align-items: stretch;
    }

    .lsp-summary {
      grid-template-columns: 1fr;
    }

    .lsp-card__header,
    .lsp-card__grid {
      grid-template-columns: 1fr;
      flex-direction: column;
    }
  }
</style>
