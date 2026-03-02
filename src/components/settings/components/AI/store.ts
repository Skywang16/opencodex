import { aiApi } from '@/api'
import type { AIModelConfig } from '@/types/domain/ai'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export const useAISettingsStore = defineStore('ai-settings', () => {
  const models = ref<AIModelConfig[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const dataVersion = ref(0)
  const isInitialized = ref(false)

  // ── Computed ────────────────────────────────────────────────────────────

  const chatModels = computed(() => models.value.filter(m => m.modelType === 'chat'))
  const embeddingModels = computed(() => models.value.filter(m => m.modelType === 'embedding'))
  const hasModels = computed(() => chatModels.value.length > 0)

  // ── Load ────────────────────────────────────────────────────────────────

  const loadSettings = async (forceRefresh = false) => {
    if (isInitialized.value && !forceRefresh) return
    isLoading.value = true
    error.value = null
    try {
      models.value = await aiApi.getModels()
      dataVersion.value++
      isInitialized.value = true
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  const reload = () => loadSettings(true)

  // ── Model CRUD ───────────────────────────────────────────────────────────

  const addModel = async (model: AIModelConfig): Promise<AIModelConfig> => {
    const result = await aiApi.addModel(model)
    await reload()
    return result
  }

  const updateModel = async (model: AIModelConfig): Promise<AIModelConfig> => {
    const result = await aiApi.updateModel(model)
    await reload()
    return result
  }

  const removeModel = async (modelId: string) => {
    await aiApi.deleteModel(modelId)
    await reload()
  }

  const clearError = () => {
    error.value = null
  }

  return {
    models,
    chatModels,
    embeddingModels,
    hasModels,
    isLoading,
    error,
    dataVersion,
    isInitialized,
    loadSettings,
    reload,
    addModel,
    updateModel,
    removeModel,
    clearError,
  }
})
