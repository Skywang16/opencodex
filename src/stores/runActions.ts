import { workspaceApi, type RunActionRecord } from '@/api/workspace'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export type { RunActionRecord }

export const useRunActionsStore = defineStore('runActions', () => {
  const actions = ref<RunActionRecord[]>([])
  const selectedActionId = ref<string | null>(null)
  const currentPath = ref<string | null>(null)

  const selectedAction = computed(() => {
    if (!selectedActionId.value) return null
    return actions.value.find(a => a.id === selectedActionId.value) ?? null
  })

  const load = async (workspacePath: string | null) => {
    currentPath.value = workspacePath
    if (!workspacePath) {
      actions.value = []
      selectedActionId.value = null
      return
    }
    actions.value = await workspaceApi.listRunActions(workspacePath)
  }

  const syncSelectedFromWorkspace = (selectedId: string | null | undefined) => {
    selectedActionId.value = selectedId ?? null
  }

  const addAction = async (name: string, command: string): Promise<RunActionRecord | null> => {
    if (!currentPath.value) return null
    const record = await workspaceApi.createRunAction(currentPath.value, name, command)
    actions.value = [...actions.value, record]
    return record
  }

  const updateAction = async (id: string, name: string, command: string) => {
    await workspaceApi.updateRunAction(id, name, command)
    actions.value = actions.value.map(a => (a.id === id ? { ...a, name, command } : a))
  }

  const deleteAction = async (id: string) => {
    await workspaceApi.deleteRunAction(id)
    actions.value = actions.value.filter(a => a.id !== id)
    if (selectedActionId.value === id) {
      const next = actions.value.length > 0 ? actions.value[0].id : null
      await selectAction(next)
    }
  }

  const selectAction = async (id: string | null) => {
    selectedActionId.value = id
    if (currentPath.value) {
      await workspaceApi.setSelectedRunAction(currentPath.value, id)
    }
  }

  return {
    actions,
    selectedActionId,
    selectedAction,
    load,
    syncSelectedFromWorkspace,
    addAction,
    updateAction,
    deleteAction,
    selectAction,
  }
})
