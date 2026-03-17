<script setup lang="ts">
  import type { Block } from '@/types'
  import { computed, ref } from 'vue'

  type ToolBlock = Extract<Block, { type: 'tool' }>

  interface Props {
    blocks: ToolBlock[]
  }

  const props = defineProps<Props>()
  const expanded = ref(true)

  type ExploreItem = {
    key: string
    label: 'Search' | 'Read' | 'List'
    text: string
  }

  const normalizeText = (value: unknown): string => {
    if (typeof value === 'string') return value.trim()
    if (typeof value === 'number' || typeof value === 'boolean') return String(value)
    return ''
  }

  const summarizeTool = (block: ToolBlock): ExploreItem | null => {
    const input = (block.input as Record<string, unknown> | null) ?? {}

    switch (block.name) {
      case 'grep':
      case 'semantic_search':
      case 'web_search': {
        const query =
          normalizeText(input.query) ||
          normalizeText(input.pattern) ||
          normalizeText(input.q) ||
          normalizeText(input.prompt)
        const path = normalizeText(input.path) || normalizeText(input.cwd)
        return {
          key: block.id,
          label: 'Search',
          text: query && path ? `${query} in ${path}` : query || path || block.name,
        }
      }
      case 'read_file':
      case 'read_terminal':
      case 'web_fetch': {
        const target =
          normalizeText(input.path) ||
          normalizeText(input.url) ||
          normalizeText(input.file_path) ||
          block.name
        return {
          key: block.id,
          label: 'Read',
          text: target,
        }
      }
      case 'list_files':
      case 'glob': {
        const target =
          normalizeText(input.path) ||
          normalizeText(input.pattern) ||
          normalizeText(input.directory) ||
          block.name
        return {
          key: block.id,
          label: 'List',
          text: target,
        }
      }
      default:
        return null
    }
  }

  const items = computed(() => props.blocks.map(summarizeTool).filter((item): item is ExploreItem => item !== null))

  const searchCount = computed(() => items.value.filter(item => item.label === 'Search').length)
  const readTargets = computed(() => items.value.filter(item => item.label === 'Read').map(item => item.text))

  const title = computed(() => {
    const firstRead = readTargets.value[0]
    if (firstRead && searchCount.value > 0) {
      return `Explored ${firstRead} and searched ${searchCount.value} quer${searchCount.value === 1 ? 'y' : 'ies'}`
    }
    if (firstRead) {
      return `Explored ${firstRead}`
    }
    if (searchCount.value > 0) {
      return `Explored ${searchCount.value} quer${searchCount.value === 1 ? 'y' : 'ies'}`
    }
    return 'Explored'
  })
</script>

<template>
  <section class="explored-group step-block">
    <button
      type="button"
      class="explored-header"
      :aria-expanded="expanded"
      :aria-label="expanded ? 'Collapse explored details' : 'Expand explored details'"
      @click="expanded = !expanded"
    >
      <span class="explored-title">{{ title }}</span>
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

    <transition name="expand">
      <div v-if="expanded" class="explored-list">
        <div v-for="item in items" :key="item.key" class="explored-row">
          <span class="explored-kind">{{ item.label }}</span>
          <span class="explored-text">{{ item.text }}</span>
        </div>
      </div>
    </transition>
  </section>
</template>

<style scoped>
  .explored-group {
    margin-bottom: var(--spacing-md);
    padding: 12px 14px;
    border-radius: 16px;
    border: 1px solid color-mix(in srgb, var(--border-200) 85%, transparent);
    background: transparent;
  }

  .explored-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .explored-title {
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-200);
    line-height: 1.5;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.18s ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .explored-list {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .explored-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    font-size: 12px;
    line-height: 1.45;
  }

  .explored-kind {
    flex-shrink: 0;
    color: var(--color-primary);
    font-weight: 600;
  }

  .explored-text {
    min-width: 0;
    color: var(--text-300);
    word-break: break-word;
  }

  @media (max-width: 720px) {
    .explored-group {
      padding: 11px 12px;
    }
  }
</style>
