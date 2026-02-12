/**
 * Shortcut state management Store
 *
 * Uses Pinia to manage reactive state and operations for shortcut configuration
 */

import { shortcutsApi } from '@/api'
import type {
  ConflictDetectionResult,
  Platform,
  ShortcutBinding,
  ShortcutsConfig,
  ShortcutStatistics,
  ShortcutValidationResult,
} from '@/types'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

export const useShortcutStore = defineStore('shortcuts', () => {
  // Simplified state: flattened structure
  const config = ref<ShortcutsConfig | null>(null)
  const currentPlatform = ref<Platform | null>(null)
  const statistics = ref<ShortcutStatistics | null>(null)
  const lastValidation = ref<ShortcutValidationResult | null>(null)
  const lastConflictDetection = ref<ConflictDetectionResult | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const initialized = ref(false)

  // Simplified computed properties
  const hasConfig = computed(() => config.value !== null)
  const hasConflicts = computed(() => lastConflictDetection.value?.has_conflicts ?? false)
  const hasValidationErrors = computed(() => lastValidation.value && !lastValidation.value.is_valid)
  const totalShortcuts = computed(() => statistics.value?.total_count ?? 0)

  // Operation methods

  // Generic async operation handler
  const withLoading = async <T>(operation: () => Promise<T>): Promise<T> => {
    loading.value = true
    error.value = null
    try {
      return await operation()
    } catch (err) {
      error.value = `Operation failed: ${err}`
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * Initialize shortcut Store
   */
  const initialize = async (): Promise<void> => {
    if (initialized.value) return

    return withLoading(async () => {
      // Load configuration, platform info, and statistics in parallel
      const [configData, platform, stats] = await Promise.all([
        shortcutsApi.getConfig(),
        shortcutsApi.getCurrentPlatform(),
        shortcutsApi.getStatistics(),
      ])

      config.value = configData
      currentPlatform.value = platform
      statistics.value = stats
      initialized.value = true

      // Perform validation and conflict detection on initialization
      await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
    })
  }

  /**
   * Refresh configuration
   */
  const refreshConfig = async (): Promise<void> => {
    return withLoading(async () => {
      const [configData, platform, stats] = await Promise.all([
        shortcutsApi.getConfig(),
        shortcutsApi.getCurrentPlatform(),
        shortcutsApi.getStatistics(),
      ])

      config.value = configData
      currentPlatform.value = platform
      statistics.value = stats
      initialized.value = true

      // Re-validate after refresh
      await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
    })
  }

  /**
   * Validate current configuration
   */
  const validateCurrentConfig = async (): Promise<ShortcutValidationResult> => {
    if (!config.value) {
      throw new Error('No configuration available for validation')
    }

    const result = await shortcutsApi.validateConfig(config.value)
    lastValidation.value = result
    return result
  }

  /**
   * Detect conflicts in current configuration
   */
  const detectCurrentConflicts = async (): Promise<ConflictDetectionResult> => {
    if (!config.value) {
      throw new Error('No configuration available for detection')
    }

    const result = await shortcutsApi.detectConflicts(config.value)
    lastConflictDetection.value = result
    return result
  }

  /**
   * Add shortcut
   */
  const addShortcut = async (shortcut: ShortcutBinding): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.addShortcut(shortcut)
      await refreshConfig()
    })
  }

  /**
   * Remove shortcut
   */
  const removeShortcut = async (index: number): Promise<ShortcutBinding> => {
    return withLoading(async () => {
      const removedShortcut = await shortcutsApi.removeShortcut(index)
      await refreshConfig()
      return removedShortcut
    })
  }

  /**
   * Update shortcut
   */
  const updateShortcut = async (index: number, shortcut: ShortcutBinding): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.updateShortcut(index, shortcut)
      await refreshConfig()
    })
  }

  /**
   * Reset to default configuration
   */
  const resetToDefaults = async (): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.resetToDefaults()
      await refreshConfig()
    })
  }

  // Watch configuration changes and automatically re-validate
  watch(
    () => config.value,
    async newConfig => {
      if (newConfig && initialized.value) {
        try {
          await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
        } catch (err) {
          console.error('Shortcut configuration validation failed:', err)
        }
      }
    },
    { deep: true }
  )

  return {
    config,
    currentPlatform,
    statistics,
    lastValidation,
    lastConflictDetection,
    loading,
    error,
    initialized,

    hasConfig,
    hasConflicts,
    hasValidationErrors,
    totalShortcuts,

    // Operation methods
    initialize,
    refreshConfig,
    validateCurrentConfig,
    detectCurrentConflicts,
    addShortcut,
    removeShortcut,
    updateShortcut,
    resetToDefaults,
    clearError: () => {
      error.value = null
    },
  }
})
