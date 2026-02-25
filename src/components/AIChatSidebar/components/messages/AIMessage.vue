<script setup lang="ts">
  import type { Block, Message } from '@/types'
  import { renderMarkdown } from '@/utils/markdown'
  import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from '../../store'
  import AgentSwitchBlock from './blocks/AgentSwitchBlock.vue'
  import SubtaskBlock from './blocks/SubtaskBlock.vue'
  import ThinkingBlock from './blocks/ThinkingBlock.vue'
  import ToolBlock from './blocks/ToolBlock.vue'
  const { t } = useI18n()
  const aiChatStore = useAIChatStore()

  interface Props {
    message: Message
    disableToolExpand?: boolean
  }

  const props = defineProps<Props>()

  const normalizeBlockType = (block: Block): Block => {
    const type = block.type.trim()
    if (type === block.type) return block
    return { ...block, type } as Block
  }

  const blocks = computed<Block[]>(() => props.message.blocks.map(normalizeBlockType))

  const STREAMING_HINTS = [
    'Thinking...',
    'Reasoning...',
    'Working on it...',
    'Planning next step...',
    'Figuring it out...',
    'Putting it together...',
    'Drafting a response...',
    'Almost there...',
  ]
  const HINT_DELAY_MS = 3000
  const HINT_ROTATE_MIN_MS = 4000
  const HINT_ROTATE_MAX_MS = 8000

  const hintIndex = ref(0)
  const showHint = ref(false)
  let delayTimer: ReturnType<typeof setTimeout> | null = null
  let rotateTimer: ReturnType<typeof setTimeout> | null = null

  const currentHint = computed(() => STREAMING_HINTS[hintIndex.value % STREAMING_HINTS.length])
  const isStreaming = computed(() => props.message.status === 'streaming')

  const resetHintDelay = () => {
    showHint.value = false
    hintIndex.value = Math.floor(Math.random() * STREAMING_HINTS.length)
    if (delayTimer) clearTimeout(delayTimer)
    if (isStreaming.value) {
      delayTimer = setTimeout(() => {
        showHint.value = true
      }, HINT_DELAY_MS)
    }
  }

  watch(() => blocks.value.length, resetHintDelay)
  watch(isStreaming, streaming => {
    if (streaming) {
      resetHintDelay()
    } else {
      showHint.value = false
      if (delayTimer) clearTimeout(delayTimer)
    }
  })

  const scheduleNextRotation = () => {
    const delay = HINT_ROTATE_MIN_MS + Math.random() * (HINT_ROTATE_MAX_MS - HINT_ROTATE_MIN_MS)
    rotateTimer = setTimeout(() => {
      hintIndex.value = (hintIndex.value + 1) % STREAMING_HINTS.length
      scheduleNextRotation()
    }, delay)
  }

  onMounted(() => {
    resetHintDelay()
    scheduleNextRotation()
  })
  onBeforeUnmount(() => {
    if (delayTimer) clearTimeout(delayTimer)
    if (rotateTimer) clearTimeout(rotateTimer)
  })

  const handleMessageClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement
    const copyBtn = target.closest('.code-copy-btn')

    if (copyBtn) {
      const wrapper = copyBtn.closest('.code-block-wrapper')
      const codeElement = wrapper?.querySelector('code')

      if (codeElement && codeElement.textContent) {
        try {
          await navigator.clipboard.writeText(codeElement.textContent)

          // Temporarily switch icon to success state
          const originalHTML = copyBtn.innerHTML
          copyBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: var(--color-success)">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          `
          setTimeout(() => {
            copyBtn.innerHTML = originalHTML
          }, 2000)
        } catch (err) {
          console.error('Failed to copy code:', err)
        }
      }
    }
  }
</script>

<template>
  <div class="ai-message">
    <div v-if="message.isSummary" class="summary-message">
      <div v-if="message.status === 'streaming'" class="summary-loading">Compressing…</div>
      <div v-else class="summary-divider" aria-hidden="true"></div>
    </div>

    <template v-else-if="blocks.length > 0">
      <div
        v-for="(block, index) in blocks"
        :key="('id' in block && block.id) || `${message.id}-${block.type}-${index}`"
        :class="{ 'block-fade-in': isStreaming }"
      >
        <ThinkingBlock v-if="block.type === 'thinking'" :block="block" :disable-expand="disableToolExpand" />

        <ToolBlock
          v-else-if="block.type === 'tool' && block.name !== 'task'"
          :block="block"
          :disable-expand="disableToolExpand"
        />

        <!-- `task` is orchestration-only; don't render it as a normal tool block -->
        <template v-else-if="block.type === 'tool' && block.name === 'task'"></template>

        <AgentSwitchBlock v-else-if="block.type === 'agent_switch'" :block="block" />

        <SubtaskBlock v-else-if="block.type === 'subtask'" :block="block" />

        <div v-else-if="block.type === 'user_text'" class="ai-message-text step-block" @click="handleMessageClick">
          <div v-html="renderMarkdown(block.content)"></div>
        </div>

        <div v-else-if="block.type === 'user_image'" class="ai-message-text step-block">
          <img :src="block.dataUrl" :alt="block.fileName || 'image'" style="max-width: 100%; border-radius: 8px" />
        </div>

        <div v-else-if="block.type === 'text'" class="ai-message-text step-block" @click="handleMessageClick">
          <div v-html="renderMarkdown(block.content)"></div>
        </div>

        <div v-else-if="block.type === 'error'" class="error-inline step-block">
          <span class="error-message">{{ block.message }}</span>
        </div>

        <div v-else class="unknown-step step-block">
          <div class="unknown-header">
            <span class="unknown-icon">❓</span>
            <span class="unknown-label">Unknown block type: {{ block.type }}</span>
          </div>
        </div>
      </div>
    </template>

    <div v-if="isStreaming && showHint && !aiChatStore.retryStatus" class="streaming-hint">
      <span class="streaming-hint-text">{{ currentHint }}</span>
    </div>

    <div v-if="isStreaming && aiChatStore.retryStatus" class="empty-message-status">
      <div class="retry-status">
        <span class="retry-label">
          {{ t('message.reconnecting', 'Reconnecting...') }} {{ aiChatStore.retryStatus.attempt }}/{{
            aiChatStore.retryStatus.maxAttempts
          }}
        </span>
        <span class="retry-error">{{ aiChatStore.retryStatus.errorMessage }}</span>
      </div>
    </div>

    <div v-if="blocks.length === 0 && !message.isSummary && !isStreaming" class="empty-message-status">
      {{ t('message.empty_response', 'No response from model') }}
    </div>
  </div>
</template>

<style scoped>
  .ai-message {
    padding: var(--spacing-md) 0;
    width: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .summary-message {
    margin: 10px 0;
    width: 100%;
  }

  .summary-loading {
    width: 100%;
    padding: 10px 0;
    text-align: center;
    font-size: var(--font-size-sm);
    color: var(--text-400);
  }

  .summary-divider {
    padding: 8px 0;
    text-align: center;
    font-size: var(--font-size-xs);
    color: var(--text-500);
  }

  .block-fade-in {
    animation: block-enter 0.3s ease-out;
  }

  @keyframes block-enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .step-block {
    margin-bottom: var(--spacing-sm);
  }

  .step-block:last-of-type {
    margin-bottom: 0;
  }

  .error-inline {
    font-size: var(--font-size-sm);
    line-height: 1.6;
  }

  .error-message {
    color: color-mix(in srgb, var(--color-error) 60%, var(--text-400));
  }

  .streaming-hint {
    padding: 4px 0;
    font-size: 13px;
    line-height: 1.6;
  }

  .streaming-hint-text {
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
    animation: hint-scan 2s linear infinite;
  }

  @keyframes hint-scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .empty-message-status {
    padding: 6px 0;
    font-size: var(--font-size-sm);
    color: var(--text-400);
    line-height: 1.4;
  }

  .retry-status {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .retry-label {
    color: var(--text-300);
    font-weight: 500;
  }

  .retry-error {
    color: color-mix(in srgb, var(--color-error) 60%, var(--text-400));
    font-size: var(--font-size-xs);
    word-break: break-word;
  }

  .ai-message-text {
    font-size: var(--font-size-md);
    line-height: 1.6;
    color: var(--text-200);
    overflow: hidden;
    min-width: 0;
    word-wrap: break-word;
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .ai-message-text :deep(p) {
    margin: var(--spacing-sm) 0;
    font-size: 1em;
  }

  .ai-message-text :deep(p:first-child) {
    margin-top: 0;
  }

  .ai-message-text :deep(p:last-child) {
    margin-bottom: 0;
  }

  /* Headings */
  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2),
  .ai-message-text :deep(h3),
  .ai-message-text :deep(h4),
  .ai-message-text :deep(h5),
  .ai-message-text :deep(h6) {
    margin: var(--spacing-md) 0 var(--spacing-sm) 0;
    font-weight: 600;
    line-height: 1.5;
    color: var(--text-100);
    letter-spacing: 0.02em;
  }

  .ai-message-text :deep(h1) {
    font-size: 1.3em;
    border-bottom: 1px solid var(--border-200);
    padding-bottom: var(--spacing-xs);
  }

  .ai-message-text :deep(h2) {
    font-size: 1.15em;
  }

  .ai-message-text :deep(h3) {
    font-size: 1.05em;
  }

  .ai-message-text :deep(h4) {
    font-size: 0.95em;
  }

  /* Code */
  .ai-message-text :deep(code) {
    font-family: var(--font-family-mono);
    font-size: 0.85em;
    padding: 0.2em 0.4em;
    background: var(--bg-300);
    border-radius: var(--border-radius-sm);
    color: var(--text-100);
    -webkit-font-smoothing: auto;
  }

  /* --- New code block style (Card Style) --- */
  .ai-message-text :deep(.code-block-wrapper) {
    margin: var(--spacing-md) 0;
    border-radius: var(--border-radius-lg);
    overflow: hidden;
    background: var(--bg-100); /* Use theme background color */
    border: 1px solid var(--border-300);
  }

  /* Code block header */
  .ai-message-text :deep(.code-block-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-300); /* Use theme variables */
    border-bottom: 1px solid var(--border-200);
  }

  .ai-message-text :deep(.code-lang) {
    font-size: 12px;
    color: var(--text-400);
    text-transform: lowercase;
    font-family: var(--font-family-mono);
  }

  .ai-message-text :deep(.code-copy-btn) {
    background: transparent;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--border-radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .ai-message-text :deep(.code-copy-btn:hover) {
    color: var(--text-200);
    background: var(--color-hover); /* Use theme variables */
  }

  /* Override pre styles */
  .ai-message-text :deep(pre) {
    margin: 0 !important;
    padding: 12px !important;
    background: transparent !important;
    border: none !important;
    border-radius: 0 !important;
    overflow-x: auto;
  }

  .ai-message-text :deep(pre code) {
    background: transparent !important;
    padding: 0 !important;
    border: none !important;
    font-family: var(--font-family-mono);
    font-size: 13px !important;
    line-height: 1.6 !important;
    color: var(--text-200); /* Use theme variables */
  }

  /* --- New table style --- */
  .ai-message-text :deep(.table-wrapper) {
    margin: var(--spacing-md) 0;
    overflow-x: auto;
    border-radius: var(--border-radius-md);
    border: 1px solid var(--border-300);
  }

  .ai-message-text :deep(table) {
    margin: 0;
    border-collapse: collapse;
    width: 100%;
    border: none;
  }

  .ai-message-text :deep(th) {
    background: var(--bg-400); /* Use theme variables */
    color: var(--text-200);
    font-weight: 600;
    padding: 10px 16px;
    border: none;
    border-bottom: 1px solid var(--border-300);
    text-align: left;
    font-size: 0.9em;
  }

  .ai-message-text :deep(td) {
    padding: 10px 16px;
    border: none;
    border-bottom: 1px solid var(--border-200);
    color: var(--text-300);
    font-size: 0.9em;
  }

  .ai-message-text :deep(tr:last-child td) {
    border-bottom: none;
  }

  /* Heading enhancement */
  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2) {
    margin-top: 24px;
    margin-bottom: 16px;
    color: var(--text-100);
    letter-spacing: -0.01em;
  }

  .ai-message-text :deep(h3) {
    margin-top: 20px;
    margin-bottom: 12px;
  }

  /* Lists */
  .ai-message-text :deep(ul),
  .ai-message-text :deep(ol) {
    margin: var(--spacing-sm) 0;
    padding-left: 1.5em;
    line-height: 1.6;
  }

  .ai-message-text :deep(li) {
    margin: var(--spacing-sm) 0;
  }

  .ai-message-text :deep(li > p) {
    margin: var(--spacing-xs) 0;
  }

  .ai-message-text :deep(li::marker) {
    color: var(--text-400);
  }

  /* Blockquote */
  .ai-message-text :deep(blockquote) {
    margin: var(--spacing-sm) 0;
    padding: var(--spacing-xs) var(--spacing-md);
    border-left: 3px solid var(--border-300);
    background: var(--bg-200);
    color: var(--text-300);
    border-radius: var(--border-radius-sm);
  }

  .ai-message-text :deep(blockquote p) {
    margin: var(--spacing-xs) 0;
  }

  /* Links */
  .ai-message-text :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  .ai-message-text :deep(a:hover) {
    text-decoration: underline;
  }

  /* Horizontal rule */
  .ai-message-text :deep(hr) {
    margin: var(--spacing-md) 0;
    border: none;
    border-top: 1px solid var(--border-200);
  }

  /* Images */
  .ai-message-text :deep(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--border-radius-md);
    margin: var(--spacing-sm) 0;
  }

  /* Emphasis */
  .ai-message-text :deep(strong) {
    font-weight: 600;
    color: var(--text-100);
  }

  .ai-message-text :deep(em) {
    font-style: italic;
  }

  /* Strikethrough and underline */
  .ai-message-text :deep(del) {
    text-decoration: line-through;
    opacity: 0.7;
  }

  .ai-message-text :deep(u) {
    text-decoration: underline;
  }
</style>
