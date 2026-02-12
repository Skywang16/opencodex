/**
 * Configuration management composable API
 *
 * Provides reactive configuration management functionality, including config retrieval, updates, reload, etc.
 */

import { configApi } from '@/api'
import { type AppConfig } from '@/api/config'
import { computed, readonly, ref } from 'vue'

/**
 * Configuration loading state
 */
export interface ConfigLoadingState {
  loading: boolean
  error: string | null
  lastUpdated: Date | null
}

/**
 * Main configuration management composable function
 */
export const useConfig = () => {
  // Reactive state
  const config = ref<AppConfig | null>(null)
  const loadingState = ref<ConfigLoadingState>({
    loading: false,
    error: null,
    lastUpdated: null,
  })

  const isLoaded = computed(() => config.value !== null)
  const hasError = computed(() => loadingState.value.error !== null)
  const isLoading = computed(() => loadingState.value.loading)

  // Generic async operation handler
  const withLoading = async <T>(operation: () => Promise<T>): Promise<T> => {
    loadingState.value.loading = true
    loadingState.value.error = null
    try {
      const result = await operation()
      loadingState.value.lastUpdated = new Date()
      return result
    } finally {
      loadingState.value.loading = false
    }
  }

  // Load configuration
  const loadConfig = async () => {
    return withLoading(async () => {
      const loadedConfig = await configApi.getConfig()
      config.value = loadedConfig
      return loadedConfig
    })
  }

  // Update configuration
  const updateConfigData = async (newConfig: AppConfig) => {
    if (!config.value) {
      throw new Error('Configuration not loaded')
    }

    return withLoading(async () => {
      await configApi.setConfig(newConfig)
      config.value = newConfig
    })
  }

  // Update specific section of configuration
  const updateConfigSection = async <K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>) => {
    if (!config.value) {
      throw new Error('Configuration not loaded')
    }

    const updatedConfig = {
      ...config.value,
      [section]: {
        ...(config.value[section] as object),
        ...(updates as object),
      },
    }

    await updateConfigData(updatedConfig)
  }

  // Reset to defaults
  const resetToDefaults = async () => {
    return withLoading(async () => {
      await configApi.resetToDefaults()
      await loadConfig() // Reload configuration
    })
  }

  // Clear error
  const clearError = () => {
    loadingState.value.error = null
  }

  // Initialization method (needs to be called manually)
  const initialize = async () => {
    await loadConfig()
  }

  return {
    config: readonly(config),
    loadingState: readonly(loadingState),

    isLoaded,
    hasError,
    isLoading,

    // Methods
    initialize,
    loadConfig,
    updateConfig: updateConfigData,
    updateConfigSection,
    resetToDefaults,
    clearError,
  }
}

export default {
  useConfig,
}
