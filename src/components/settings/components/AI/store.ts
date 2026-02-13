import { aiApi } from '@/api'
import { AuthType } from '@/types/oauth'

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AIModelConfig, AISettings } from '@/types'
import type { AIModelCreateInput, AIModelUpdateInput } from '@/api/ai/types'

export const useAISettingsStore = defineStore('ai-settings', () => {
  const settings = ref<AISettings | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const dataVersion = ref(0)
  const isInitialized = ref(false)

  const hasModels = computed(() => {
    return (settings.value?.models?.length || 0) > 0
  })

  const enabledModels = computed(() => {
    return settings.value?.models || []
  })

  const models = computed(() => {
    return settings.value?.models || []
  })

  const chatModels = computed(() => {
    return models.value.filter(model => model.modelType === 'chat')
  })

  const embeddingModels = computed(() => {
    return models.value.filter(model => model.modelType === 'embedding')
  })

  const loadModels = async () => {
    isLoading.value = true
    error.value = null
    try {
      const models = await aiApi.getModels()

      if (!settings.value) {
        settings.value = {
          models,
          features: {
            chat: { enabled: true, maxHistoryLength: 1000, autoSaveHistory: true, contextWindowSize: 4000 },
          },
          performance: {
            requestTimeout: 30,
            maxConcurrentRequests: 5,
            cacheEnabled: true,
            cacheTtl: 3600,
          },
        } as AISettings
      } else {
        settings.value.models = models
      }

      dataVersion.value++
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  const loadSettings = async (forceRefresh = false) => {
    if (isInitialized.value && !forceRefresh) return

    await loadModels()
    isInitialized.value = true
  }

  const updateSettings = async (newSettings: Partial<AISettings>) => {
    if (!settings.value) {
      throw new Error('AI settings not initialized')
    }

    isLoading.value = true
    error.value = null

    const updatedSettings = { ...settings.value, ...newSettings }
    settings.value = updatedSettings
    isLoading.value = false
  }

  const addModel = async (model: AIModelCreateInput) => {
    await aiApi.addModel(model)
    await loadModels()
  }

  const updateModel = async (modelId: string, updates: Partial<AIModelConfig>) => {
    const existingModel = models.value.find(m => m.id === modelId)
    if (!existingModel) {
      throw new Error(`Model ${modelId} does not exist`)
    }

    const updatedModel = { ...existingModel, ...updates }
    const payload: AIModelUpdateInput = {
      id: modelId,
      changes: {
        provider: updatedModel.provider,
        authType: updatedModel.authType,
        apiUrl: updatedModel.apiUrl,
        apiKey: updatedModel.apiKey,
        model: updatedModel.model,
        modelType: updatedModel.modelType,
        options: updatedModel.options,
        oauthConfig: updatedModel.authType === AuthType.OAuth ? (updatedModel.oauthConfig ?? null) : null,
        useCustomBaseUrl: updatedModel.useCustomBaseUrl,
      },
    }

    await aiApi.updateModel(payload)
    await loadModels()
  }

  const removeModel = async (modelId: string) => {
    await aiApi.deleteModel(modelId)
    await loadModels()
  }

  const resetToDefaults = async () => {
    throw new Error('Reset functionality pending implementation - requires backend API support')
  }

  const clearError = () => {
    error.value = null
  }

  return {
    settings,
    isLoading,
    error,
    dataVersion,
    isInitialized,
    hasModels,
    enabledModels,
    models,
    chatModels,
    embeddingModels,
    loadSettings,
    loadModels,
    updateSettings,
    addModel,
    updateModel,
    removeModel,
    resetToDefaults,
    clearError,
  }
})
