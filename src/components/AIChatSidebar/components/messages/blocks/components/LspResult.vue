<template>
  <div class="lsp-result">
    <template v-if="action === 'status'">
      <div v-if="statusItems.length === 0" class="empty-state">No LSP servers started yet.</div>
      <div v-else class="status-grid">
        <div v-for="item in statusItems" :key="`${item.serverId}-${item.root}`" class="status-card">
          <div class="status-card__header">
            <span class="status-card__name">{{ item.serverId }}</span>
            <span class="status-pill" :class="statusClass(item)">
              {{ item.connected ? (item.initialized ? 'Ready' : 'Starting') : 'Offline' }}
            </span>
          </div>
          <div class="status-card__meta">{{ item.root }}</div>
          <div class="status-card__stats">
            <span>{{ item.openDocuments }} docs</span>
            <span>{{ item.diagnosticsFiles }} diagnostics</span>
          </div>
          <div v-if="item.lastError" class="status-card__error">{{ item.lastError }}</div>
        </div>
      </div>
    </template>

    <template v-else-if="action === 'document_symbols'">
      <div v-if="symbolRows.length === 0" class="empty-state">No symbols returned.</div>
      <div v-else class="symbol-list">
        <div v-for="row in symbolRows" :key="`${row.depth}-${row.name}-${row.range.start.line}`" class="symbol-row">
          <span class="symbol-indent" :style="{ width: `${row.depth * 14}px` }" />
          <span class="symbol-name">{{ row.name }}</span>
          <span class="symbol-detail">{{ formatRange(row.range) }}</span>
        </div>
      </div>
    </template>

    <template v-else-if="action === 'workspace_symbols' || action === 'definition' || action === 'references'">
      <div v-if="locationRows.length === 0" class="empty-state">No locations returned.</div>
      <div v-else class="location-list">
        <div v-for="row in locationRows" :key="row.key" class="location-row">
          <div class="location-row__title">{{ row.title }}</div>
          <div class="location-row__meta">{{ row.meta }}</div>
        </div>
      </div>
    </template>

    <template v-else-if="action === 'hover'">
      <div v-if="!hoverResult" class="empty-state">No hover details returned.</div>
      <div v-else class="hover-card">
        <div v-if="hoverResult.range" class="hover-range">{{ formatRange(hoverResult.range) }}</div>
        <pre class="hover-content">{{ hoverResult.contents }}</pre>
      </div>
    </template>

    <template v-else-if="action === 'diagnostics'">
      <div v-if="diagnosticGroups.length === 0" class="empty-state">No diagnostics.</div>
      <div v-else class="diagnostic-groups">
        <div v-for="group in diagnosticGroups" :key="group.path" class="diagnostic-group">
          <div class="diagnostic-group__header">
            <span class="diagnostic-group__file">{{ group.file }}</span>
            <span class="diagnostic-group__count">{{ group.diagnostics.length }}</span>
          </div>
          <div v-for="diag in group.diagnostics" :key="diag.key" class="diagnostic-row">
            <span class="diagnostic-severity" :class="diag.level" />
            <div class="diagnostic-row__body">
              <div class="diagnostic-row__message">{{ diag.message }}</div>
              <div class="diagnostic-row__meta">{{ diag.location }}</div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <pre v-else class="fallback">{{ resultText }}</pre>
  </div>
</template>

<script setup lang="ts">
  import type {
    LspDocumentSymbol,
    LspFileDiagnostics,
    LspHoverResult,
    LspLocation,
    LspRange,
    LspServerStatus,
    LspWorkspaceSymbol,
  } from '@/api/lsp/types'
  import { getPathBasename } from '@/utils/path'
  import { computed } from 'vue'

  const props = defineProps<{
    action: string
    metadata: unknown
    resultText: string
  }>()

  const statusItems = computed<LspServerStatus[]>(() =>
    Array.isArray(props.metadata) ? (props.metadata as LspServerStatus[]) : []
  )

  const symbolRows = computed(() => {
    const symbols = Array.isArray(props.metadata) ? (props.metadata as LspDocumentSymbol[]) : []
    const rows: Array<LspDocumentSymbol & { depth: number }> = []
    const walk = (items: LspDocumentSymbol[], depth: number) => {
      for (const item of items) {
        rows.push({ ...item, depth })
        if (item.children?.length) walk(item.children, depth + 1)
      }
    }
    walk(symbols, 0)
    return rows
  })

  const locationRows = computed(() => {
    if (!Array.isArray(props.metadata)) return []
    const rows = props.metadata as Array<LspLocation | LspWorkspaceSymbol>
    return rows.map((item, index) => {
      if ('range' in item && 'path' in item) {
        return {
          key: `${item.path}-${index}`,
          title: getPathBasename(item.path),
          meta: formatRange(item.range),
        }
      }
      return {
        key: `${item.name}-${index}`,
        title: item.name,
        meta: item.location
          ? `${getPathBasename(item.location.path)} ${formatRange(item.location.range)}`
          : 'No file location',
      }
    })
  })

  const hoverResult = computed<LspHoverResult | null>(() => {
    if (!props.metadata || Array.isArray(props.metadata) || typeof props.metadata !== 'object') return null
    return props.metadata as LspHoverResult
  })

  const diagnosticGroups = computed(() => {
    const files = Array.isArray(props.metadata) ? (props.metadata as LspFileDiagnostics[]) : []
    return files.map(file => ({
      path: file.path,
      file: getPathBasename(file.path),
      diagnostics: file.diagnostics.map((diag, index) => ({
        key: `${file.path}-${index}`,
        message: diag.message,
        location: formatRange(diag.range),
        level: severityClass(diag.severity),
      })),
    }))
  })

  const formatRange = (range: LspRange) => {
    return `L${range.start.line + 1}:${range.start.character + 1}`
  }

  const severityClass = (severity?: number) => {
    switch (severity) {
      case 1:
        return 'error'
      case 2:
        return 'warning'
      default:
        return 'info'
    }
  }

  const statusClass = (item: LspServerStatus) => {
    if (!item.connected) return 'offline'
    return item.initialized ? 'ready' : 'starting'
  }
</script>

<style scoped>
  .lsp-result {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 2px 0;
  }

  .empty-state,
  .fallback,
  .hover-content {
    margin: 0;
    color: var(--text-400);
    font-family: var(--font-family-mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .status-grid,
  .diagnostic-groups,
  .symbol-list,
  .location-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .status-card,
  .hover-card,
  .diagnostic-group {
    border: 1px solid var(--border-200);
    background: var(--bg-100);
    border-radius: var(--border-radius-lg);
    padding: 10px 12px;
  }

  .status-card__header,
  .diagnostic-group__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 6px;
  }

  .status-card__name,
  .diagnostic-group__file,
  .symbol-name,
  .location-row__title {
    color: var(--text-200);
    font-size: 13px;
    font-weight: 600;
  }

  .status-card__meta,
  .status-card__stats,
  .location-row__meta,
  .diagnostic-row__meta,
  .hover-range,
  .symbol-detail {
    color: var(--text-500);
    font-size: 12px;
    font-family: var(--font-family-mono);
  }

  .status-card__stats {
    display: flex;
    gap: 10px;
    margin-top: 6px;
  }

  .status-card__error {
    margin-top: 8px;
    color: var(--color-error);
    font-size: 12px;
    line-height: 1.4;
  }

  .status-pill {
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .status-pill.ready {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .status-pill.starting {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .status-pill.offline {
    color: var(--text-500);
    background: var(--bg-300);
  }

  .symbol-row,
  .location-row,
  .diagnostic-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-md);
    background: color-mix(in srgb, var(--bg-100) 88%, var(--bg-200));
  }

  .symbol-row {
    align-items: center;
  }

  .symbol-indent {
    display: inline-block;
    flex-shrink: 0;
  }

  .symbol-name {
    flex: 1;
    min-width: 0;
  }

  .symbol-detail {
    flex-shrink: 0;
  }

  .location-row,
  .diagnostic-row {
    flex-direction: column;
    gap: 4px;
  }

  .diagnostic-group__count {
    color: var(--text-400);
    font-size: 12px;
  }

  .diagnostic-row {
    flex-direction: row;
    align-items: flex-start;
  }

  .diagnostic-severity {
    width: 8px;
    height: 8px;
    margin-top: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .diagnostic-severity.error {
    background: var(--color-error);
  }

  .diagnostic-severity.warning {
    background: var(--color-warning);
  }

  .diagnostic-severity.info {
    background: var(--color-info);
  }

  .diagnostic-row__body {
    min-width: 0;
    flex: 1;
  }

  .diagnostic-row__message {
    color: var(--text-300);
    font-size: 13px;
    line-height: 1.45;
  }
</style>
