import { aiApi, llmApi } from '@/api'
import type { AIModelConfig, PresetModel, ProviderMetadata } from '@/types'
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
  const providers = ref<ProviderMetadata[]>([])
  const userModels = ref<AIModelConfig[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Convert backend data to format used by frontend forms
  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => ({
      value: provider.providerType,
      label: provider.displayName,
      apiUrl: provider.defaultApiUrl || undefined,
    }))
  })

  // Get model options by provider type
  const getModelOptions = (providerId: string): ModelOption[] => {
    const provider = providers.value.find(p => p.providerType === providerId)
    if (!provider) return []

    return provider.presetModels.map(model => ({
      value: model.id,
      label: model.name,
      reasoning: model.capabilities.reasoning,
      toolCall: model.capabilities.toolCall,
      attachment: model.capabilities.attachment,
    }))
  }

  // Get chat model options (alias for getModelOptions)
  const getChatModelOptions = (providerId: string): ModelOption[] => {
    return getModelOptions(providerId)
  }

  // Get model info by provider and model ID
  const getModelInfo = (providerId: string, modelId: string): PresetModel | null => {
    const provider = providers.value.find(p => p.providerType === providerId)
    if (!provider) return null
    return provider.presetModels.find(m => m.id === modelId) || null
  }

  // Get provider info by provider ID
  const getProviderInfo = (providerId: string): ProviderMetadata | null => {
    return providers.value.find(p => p.providerType === providerId) || null
  }

  // Check if selected model supports reasoning
  const modelSupportsReasoning = (providerId: string, modelId: string): boolean => {
    const model = getModelInfo(providerId, modelId)
    return model?.capabilities.reasoning ?? false
  }

  // Load provider and model data from backend registry
  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      const [providersData, modelsData] = await Promise.all([llmApi.getProviders(), aiApi.getModels()])

      providers.value = providersData
      userModels.value = modelsData
    } catch (err) {
      console.error('Failed to load LLM providers:', err)
      error.value = err instanceof Error ? err.message : 'Failed to load'
    } finally {
      isLoading.value = false
    }
  }

  // Refresh providers from backend registry
  const refreshProviders = async () => {
    isLoading.value = true
    error.value = null

    try {
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
