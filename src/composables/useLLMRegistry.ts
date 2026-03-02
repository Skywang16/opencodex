import { aiApi, llmApi } from '@/api'
import type { AIModelConfig, ModelsDevProvider, PresetModel, ProviderMetadata } from '@/types'
import { computed, ref } from 'vue'

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

// Map hardcoded providerType → models.dev provider ID
const PROVIDER_TO_MODELS_DEV: Record<string, string> = {
  anthropic: 'anthropic',
  openai: 'openai',
  gemini: 'google',
}

// Merge models.dev model list into a hardcoded provider's presetModels
function mergeModelsDevIntoProvider(provider: ProviderMetadata, devProviders: ModelsDevProvider[]): ProviderMetadata {
  const devId = PROVIDER_TO_MODELS_DEV[provider.providerType]
  if (!devId) return provider

  const devProvider = devProviders.find(p => p.id === devId)
  if (!devProvider || devProvider.models.length === 0) return provider

  return {
    ...provider,
    presetModels: devProvider.models.map(m => ({
      id: m.id,
      name: m.name,
      maxTokens: m.maxOutput || undefined,
      contextWindow: m.contextWindow || 128000,
      capabilities: {
        reasoning: m.reasoning,
        toolCall: m.toolCall,
        attachment: m.attachment,
      },
    })),
  }
}

export const useLLMRegistry = () => {
  const providers = ref<ProviderMetadata[]>([])
  const userModels = ref<AIModelConfig[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const dataSource = ref<'models_dev' | 'preset'>('preset')

  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => ({
      value: provider.providerType,
      label: provider.displayName,
      apiUrl: provider.defaultApiUrl || undefined,
    }))
  })

  const findProvider = (providerId: string): ProviderMetadata | undefined => {
    return providers.value.find(p => p.providerType === providerId)
  }

  // Get model options by provider ID
  const getModelOptions = (providerId: string): ModelOption[] => {
    const provider = findProvider(providerId)
    if (!provider) return []

    return provider.presetModels.map(model => ({
      value: model.id,
      label: model.name,
      reasoning: model.capabilities.reasoning,
      toolCall: model.capabilities.toolCall,
      attachment: model.capabilities.attachment,
    }))
  }

  const getChatModelOptions = (providerId: string): ModelOption[] => {
    return getModelOptions(providerId)
  }

  const getModelInfo = (providerId: string, modelId: string): PresetModel | null => {
    const provider = findProvider(providerId)
    if (!provider) return null
    return provider.presetModels.find(m => m.id === modelId) || null
  }

  const getProviderInfo = (providerId: string): ProviderMetadata | null => {
    return findProvider(providerId) || null
  }

  const modelSupportsReasoning = (providerId: string, modelId: string): boolean => {
    const model = getModelInfo(providerId, modelId)
    return model?.capabilities.reasoning ?? false
  }

  // Load providers from hardcoded registry, then replace model lists with models.dev data
  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      const [presetProviders, devProviders, modelsData] = await Promise.all([
        llmApi.getProviders(),
        llmApi.getModelsDevProviders().catch(err => {
          console.warn('models.dev fetch failed, using preset models:', err)
          return null
        }),
        aiApi.getModels(),
      ])

      if (devProviders && devProviders.length > 0) {
        // Providers from hardcoded registry, models from models.dev
        providers.value = presetProviders.map(p => mergeModelsDevIntoProvider(p, devProviders))
        dataSource.value = 'models_dev'
      } else {
        // Pure fallback: everything from hardcoded registry
        providers.value = presetProviders
        dataSource.value = 'preset'
      }

      userModels.value = modelsData
    } catch (err) {
      console.error('Failed to load LLM providers:', err)
      error.value = err instanceof Error ? err.message : 'Failed to load'
    } finally {
      isLoading.value = false
    }
  }

  // Force refresh model lists from models.dev
  const refreshProviders = async () => {
    isLoading.value = true
    error.value = null

    try {
      await llmApi.refreshModelsDev()
      isLoading.value = false
      await loadProviders()
    } catch (err) {
      console.error('Failed to refresh providers:', err)
      error.value = err instanceof Error ? err.message : 'Failed to refresh'
      isLoading.value = false
    }
  }

  return {
    providers,
    userModels,
    providerOptions,
    isLoading,
    error,
    dataSource,

    loadProviders,
    refreshProviders,
    findProvider,
    getModelOptions,
    getChatModelOptions,
    getModelInfo,
    getProviderInfo,
    modelSupportsReasoning,
  }
}
