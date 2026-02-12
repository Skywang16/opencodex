import { aiApi, llmApi } from '@/api'
import type { AIModelConfig, ModelsDevModelInfo, ModelsDevProviderInfo } from '@/types'
import { computed, onMounted, ref } from 'vue'

export interface ProviderOption {
  value: string
  label: string
  apiUrl?: string
}

export interface ModelOption {
  value: string
  label: string
  reasoning: boolean
  toolCall: boolean
  attachment: boolean
}

export const useLLMRegistry = () => {
  const providers = ref<ModelsDevProviderInfo[]>([])
  const userModels = ref<AIModelConfig[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Convert backend data to format used by frontend forms
  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => ({
      value: provider.id,
      label: provider.name,
      apiUrl: provider.apiUrl,
    }))
  })

  // Get model options by provider type
  const getModelOptions = (providerId: string): ModelOption[] => {
    const provider = providers.value.find(p => p.id === providerId)
    if (!provider) return []

    return provider.models.map(model => ({
      value: model.id,
      label: model.name,
      reasoning: model.reasoning,
      toolCall: model.toolCall,
      attachment: model.attachment,
    }))
  }

  // Get chat model options (alias for getModelOptions)
  const getChatModelOptions = (providerId: string): ModelOption[] => {
    return getModelOptions(providerId)
  }

  // Get model info by provider and model ID
  const getModelInfo = (providerId: string, modelId: string): ModelsDevModelInfo | null => {
    const provider = providers.value.find(p => p.id === providerId)
    if (!provider) return null
    return provider.models.find(m => m.id === modelId) || null
  }

  // Get provider info by provider ID
  const getProviderInfo = (providerId: string): ModelsDevProviderInfo | null => {
    return providers.value.find(p => p.id === providerId) || null
  }

  // Check if selected model supports reasoning
  const modelSupportsReasoning = (providerId: string, modelId: string): boolean => {
    const model = getModelInfo(providerId, modelId)
    return model?.reasoning ?? false
  }

  // Load provider and model data from models.dev API
  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      const [providersData, modelsData] = await Promise.all([llmApi.getModelsDevProviders(), aiApi.getModels()])

      providers.value = providersData
      userModels.value = modelsData
    } catch (err) {
      console.error('Failed to load LLM providers:', err)
      error.value = err instanceof Error ? err.message : 'Failed to load'
    } finally {
      isLoading.value = false
    }
  }

  // Refresh providers from models.dev API
  const refreshProviders = async () => {
    isLoading.value = true
    error.value = null

    try {
      await llmApi.refreshModelsDev()
      await loadProviders()
    } catch (err) {
      console.error('Failed to refresh providers:', err)
      error.value = err instanceof Error ? err.message : 'Failed to refresh'
    } finally {
      isLoading.value = false
    }
  }

  // Auto-load data
  onMounted(() => {
    loadProviders()
  })

  return {
    // Reactive data
    providers,
    userModels,
    providerOptions,
    isLoading,
    error,

    // Methods
    loadProviders,
    refreshProviders,
    getModelOptions,
    getChatModelOptions,
    getModelInfo,
    getProviderInfo,
    modelSupportsReasoning,
  }
}
