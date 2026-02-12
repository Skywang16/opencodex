import { useTerminalStore } from '@/stores/Terminal'
import {
  TagType,
  type TagContextInfo,
  type TagState,
  type TerminalSelectionTag,
  type TerminalTabTag,
} from '@/types/tags'
import { getPathBasename } from '@/utils/path'
import { computed, ref } from 'vue'

interface TerminalSelection {
  text: string
  startLine: number
  endLine: number
  path?: string
}

const selectedTerminalData = ref<TerminalSelection | null>(null)

export const useTerminalSelection = () => {
  const terminalStore = useTerminalStore()

  // Computed properties - auto respond to data changes
  const hasSelection = computed(() => !!selectedTerminalData.value)
  const selectedText = computed(() => selectedTerminalData.value?.text ?? '')

  const selectionInfo = computed(() => {
    const data = selectedTerminalData.value
    if (!data) return ''

    const { startLine, endLine, path } = data

    const pathDisplay = path ? getPathBasename(path) : 'terminal'

    return startLine === endLine ? `${pathDisplay} ${startLine}:${startLine}` : `${pathDisplay} ${startLine}:${endLine}`
  })

  // Terminal tab computed properties - based on active terminal
  const currentTerminalTab = computed(() => {
    const activeTerminal = terminalStore.activeTerminal
    if (!activeTerminal) return null

    return {
      terminalId: activeTerminal.id,
      shell: activeTerminal.shell,
      cwd: activeTerminal.cwd,
      displayPath: getPathBasename(activeTerminal.cwd),
    }
  })

  const hasTerminalTab = computed(() => !!currentTerminalTab.value)

  // Set selected text - simplified logic
  const setSelectedText = (text: string, startLine = 1, endLine?: number, path?: string) => {
    if (!text.trim()) {
      selectedTerminalData.value = null
      return
    }

    const lineCount = text.split('\n').length
    const actualEndLine = endLine ?? startLine + lineCount - 1

    selectedTerminalData.value = { text, startLine, endLine: actualEndLine, path }
  }

  // Clear selection
  const clearSelection = () => {
    selectedTerminalData.value = null
  }

  // Tag state management
  const getTagState = (): TagState => {
    const terminalSelectionTag: TerminalSelectionTag | null = selectedTerminalData.value
      ? {
          id: 'terminal-selection',
          type: TagType.TERMINAL_SELECTION,
          removable: true,
          selectedText: selectedTerminalData.value.text,
          selectionInfo: selectionInfo.value,
          startLine: selectedTerminalData.value.startLine,
          endLine: selectedTerminalData.value.endLine,
          path: selectedTerminalData.value.path,
        }
      : null

    const terminalTabTag: TerminalTabTag | null = currentTerminalTab.value
      ? {
          id: 'terminal-tab',
          type: TagType.TERMINAL_TAB,
          removable: true,
          terminalId: currentTerminalTab.value.terminalId,
          shell: currentTerminalTab.value.shell,
          cwd: currentTerminalTab.value.cwd,
          displayPath: currentTerminalTab.value.displayPath,
        }
      : null

    return {
      terminalSelection: terminalSelectionTag,
      terminalTab: terminalTabTag,
      nodeVersion: null, // Node version is managed separately by useNodeVersion
    }
  }

  const getTagContextInfo = (): TagContextInfo => {
    const result: TagContextInfo = {
      hasTerminalTab: hasTerminalTab.value,
      hasTerminalSelection: hasSelection.value,
      hasNodeVersion: false, // Node version is managed separately by useNodeVersion
    }

    if (currentTerminalTab.value) {
      result.terminalTabInfo = {
        terminalId: currentTerminalTab.value.terminalId,
        shell: currentTerminalTab.value.shell,
        cwd: currentTerminalTab.value.cwd,
      }
    }

    if (selectedTerminalData.value) {
      result.terminalSelectionInfo = {
        selectedText: selectedTerminalData.value.text,
        selectionInfo: selectionInfo.value,
        startLine: selectedTerminalData.value.startLine,
        endLine: selectedTerminalData.value.endLine,
        path: selectedTerminalData.value.path,
      }
    }

    return result
  }

  return {
    selectedText,
    hasSelection,
    selectionInfo,
    hasTerminalTab,
    currentTerminalTab,
    // Methods
    setSelectedText,
    clearSelection,
    getSelectedText: () => selectedText.value,
    // New tag management methods
    getTagState,
    getTagContextInfo,
  }
}
