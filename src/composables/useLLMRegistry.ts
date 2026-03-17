import { llmApi } from '@/api'
import type { ProviderMetadata } from '@/types'
import { computed, ref } from 'vue'

export interface ProviderOption {
  value: string
  label: string
  apiUrl?: string
}

export const useLLMRegistry = () => {
  const formatErrorMessage = (error: unknown): string => {
    return error instanceof Error ? error.message : String(error)
  }

  const providers = ref<ProviderMetadata[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => ({
      value: provider.providerType,
      label: provider.displayName,
      apiUrl: provider.defaultApiUrl === '' ? undefined : provider.defaultApiUrl,
    }))
  })

  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      providers.value = await llmApi.getProviders()
    } catch (err) {
      console.error('Failed to load LLM providers:', err)
      error.value = formatErrorMessage(err)
    } finally {
      isLoading.value = false
    }
  }

  const refreshProviders = async () => {
    await loadProviders()
  }

  return {
    providers,
    providerOptions,
    isLoading,
    error,
    loadProviders,
    refreshProviders,
  }
}
