<template>
  <div v-if="showCompletion" class="completion-suggestion" :style="completionStyle">
    <span class="completion-text">{{ completionText }}</span>
    <span class="completion-hint">{{ shortcutHint }}</span>
  </div>
</template>

<script setup lang="ts">
  import { completionApi } from '@/api'
  import type { CompletionRequest, CompletionResponse, CompletionItem } from '@/api'
  import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
  import { debounce } from 'lodash-es'

  // Props
  interface Props {
    input: string
    workingDirectory: string
    terminalElement?: HTMLElement | null
    terminalCursorPosition?: { x: number; y: number }
    isMac?: boolean
  }

  const props = defineProps<Props>()

  // Emits
  interface Emits {
    (e: 'completion-ready', items: CompletionItem[]): void
    (e: 'suggestion-change', suggestion: string): void
  }

  const emit = defineEmits<Emits>()

  const completionItems = ref<CompletionItem[]>([])
  const currentSuggestion = ref('')
  const replaceStart = ref(0)
  const replaceEnd = ref(0)
  const isLoading = ref(false)
  let currentRequest: AbortController | null = null

  const showCompletion = computed(() => {
    return props.input.length > 0 && currentSuggestion.value.length > 0 && completionText.value.length > 0
  })

  const shortcutHint = computed(() => {
    return props.isMac ? 'Cmd+→' : 'Ctrl+→'
  })

  const completionText = computed(() => {
    if (!currentSuggestion.value) return ''
    const cursorPos = props.input.length
    const start = Math.min(replaceStart.value, cursorPos)
    const offset = cursorPos - start

    const typed = props.input.slice(start, cursorPos)
    if (!currentSuggestion.value.toLowerCase().startsWith(typed.toLowerCase())) {
      return ''
    }

    return currentSuggestion.value.slice(offset)
  })

  const completionStyle = computed(() => {
    if (!props.terminalElement || !showCompletion.value) return {}

    // If terminal cursor position is passed, use it directly
    if (props.terminalCursorPosition && props.terminalCursorPosition.x > 0 && props.terminalCursorPosition.y > 0) {
      const { x, y } = props.terminalCursorPosition

      // Get terminal wrapper position (positioning context for completion component)
      const wrapperElement = props.terminalElement.parentElement
      if (!wrapperElement) return {}

      const wrapperRect = wrapperElement.getBoundingClientRect()

      // Calculate position relative to wrapper
      const relativeX = x - wrapperRect.left
      const relativeY = y - wrapperRect.top

      // Ensure completion hint does not exceed wrapper boundaries
      const maxX = wrapperRect.width - 200 // Reserve width for completion hint
      const maxY = wrapperRect.height - 30 // Reserve height for completion hint

      const finalX = Math.min(Math.max(0, relativeX), maxX) + 20
      const finalY = Math.min(Math.max(0, relativeY), maxY)

      return {
        left: `${finalX}px`,
        top: `${finalY}px`,
        zIndex: '1000',
      }
    }

    return {}
  })

  // Local completion fallback
  const getLocalCompletions = (input: string): CompletionResponse => {
    const commands = [
      'ls',
      'ls -la',
      'ls -l',
      'cd',
      'cd ..',
      'pwd',
      'mkdir',
      'touch',
      'rm',
      'rm -rf',
      'cp',
      'cp -r',
      'mv',
      'cat',
      'grep',
      'find',
      'which',
      'history',
      'clear',
      'exit',
      'git status',
      'git add',
      'git add .',
      'git commit -m',
      'git push',
      'git pull',
      'npm install',
      'npm run dev',
      'npm run build',
      'npm start',
      'yarn install',
      'yarn dev',
    ]

    const matches = commands
      .filter(cmd => cmd.toLowerCase().startsWith(input.toLowerCase()))
      .slice(0, 10)
      .map(cmd => ({
        text: cmd,
        displayText: cmd,
        description: `Command: ${cmd}`,
        kind: 'command',
        score: 1.0,
        source: 'local',
      }))

    return {
      items: matches,
      replaceStart: 0,
      replaceEnd: input.length,
      hasMore: false,
    }
  }

  // Core logic for getting completion suggestions
  const fetchCompletions = async (input: string) => {
    if (!input || input.length === 0) {
      completionItems.value = []
      currentSuggestion.value = ''
      emit('completion-ready', [])
      emit('suggestion-change', '')
      return
    }

    // Cancel previous request
    if (currentRequest) {
      currentRequest.abort()
    }

    // Create new request controller
    currentRequest = new AbortController()

    isLoading.value = true

    const request: CompletionRequest = {
      input,
      cursorPosition: input.length,
      workingDirectory: props.workingDirectory,
      maxResults: 10,
    }

    // Try calling backend API, fallback to local completion on failure
    const response = await completionApi.getCompletions(request).catch(() => {
      return getLocalCompletions(input)
    })

    completionItems.value = response.items
    emit('completion-ready', response.items)

    // Set first match as inline completion
    if (response.items.length > 0) {
      const firstItem = response.items[0]
      replaceStart.value = response.replaceStart ?? 0
      replaceEnd.value = response.replaceEnd ?? input.length
      currentSuggestion.value = firstItem.text
    } else {
      currentSuggestion.value = ''
      replaceStart.value = 0
      replaceEnd.value = 0
    }

    emit('suggestion-change', currentSuggestion.value)
    isLoading.value = false
  }

  // Completion function using lodash debounce
  const debouncedFetchCompletions = debounce(fetchCompletions, 150)

  // Watch input changes
  watch(
    () => props.input,
    newInput => {
      debouncedFetchCompletions(newInput)
    },
    { immediate: true }
  )

  /**
   * Accept current completion suggestion
   * Clear current completion state as completion has been accepted
   */
  const acceptCompletion = () => {
    const completionToAccept = completionText.value
    if (completionToAccept && completionToAccept.trim()) {
      // Clear current completion state
      currentSuggestion.value = ''
      replaceStart.value = 0
      replaceEnd.value = 0
      completionItems.value = []
      emit('suggestion-change', '')
      emit('completion-ready', [])
      return completionToAccept
    }
    return ''
  }

  /**
   * Check if there are available completion suggestions
   */
  const hasCompletion = () => showCompletion.value && !!completionText.value && completionText.value.length > 0

  // Handle completion acceptance triggered by shortcut
  const handleAcceptCompletionEvent = (event: Event) => {
    if (event.type === 'accept-completion') {
      const result = acceptCompletion()
      if (result) {
        // Trigger a custom event to let parent component (Terminal) know a completion was accepted
        const detailEvent = new CustomEvent('completion-accepted', {
          detail: { completion: result },
          bubbles: true,
        })
        event.target?.dispatchEvent(detailEvent)
      }
    }
  }

  // Add event listener
  onMounted(() => {
    if (props.terminalElement) {
      props.terminalElement.addEventListener('accept-completion', handleAcceptCompletionEvent)
    }
  })

  onUnmounted(() => {
    if (props.terminalElement) {
      props.terminalElement.removeEventListener('accept-completion', handleAcceptCompletionEvent)
    }
  })

  // Expose methods to parent component
  defineExpose({
    getCompletionText: () => completionText.value,
    acceptCompletion,
    hasCompletion,
  })
</script>

<style scoped>
  .completion-suggestion {
    position: absolute;
    pointer-events: none;
    z-index: 1000;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .completion-text {
    color: var(--text-400);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-md);
    background: var(--bg-500);
    padding: 1px 4px;
    border-radius: var(--border-radius-xs);
    border: 1px solid var(--border-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60vw;
  }

  .completion-hint {
    color: var(--text-500);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-xs);
    background: var(--bg-400);
    padding: 2px 6px;
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-200);
    opacity: 0.7;
  }
</style>
