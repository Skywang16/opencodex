import oauthApi from '@/api/llm/oauth'
import type { OAuthConfig, OAuthProvider, OAuthStatus } from '@/types/oauth'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

export interface OAuthState {
  isAuthenticating: boolean
  flowId: string | null
  cancelled: boolean
  config: OAuthConfig | null
}

export const useOAuth = () => {
  const state = ref<OAuthState>({
    isAuthenticating: false,
    flowId: null,
    cancelled: false,
    config: null,
  })

  const isAuthenticating = computed(() => state.value.isAuthenticating)
  const config = computed(() => state.value.config)

  /**
   * Start OAuth authorization flow
   */
  const startAuthorization = async (provider: OAuthProvider): Promise<OAuthConfig | null> => {
    state.value.isAuthenticating = true
    state.value.cancelled = false
    state.value.config = null

    try {
      // 1. Start OAuth flow, get authorization URL
      const flowInfo = await oauthApi.startFlow(provider)
      state.value.flowId = flowInfo.flowId

      // 2. Open authorization URL in browser
      await openUrl(flowInfo.authorizeUrl)

      // 3. Wait for callback
      const oauthConfig = await oauthApi.waitForCallback(flowInfo.flowId, provider)
      state.value.config = oauthConfig

      return oauthConfig
    } catch (err) {
      // User cancelled - return null silently
      if (state.value.cancelled) {
        return null
      }
      // Re-throw to let caller handle
      throw err
    } finally {
      state.value.isAuthenticating = false
      state.value.flowId = null
    }
  }

  /**
   * Cancel current OAuth flow
   */
  const cancelAuthorization = async (): Promise<void> => {
    if (!state.value.flowId) return

    // Mark as actively cancelled
    state.value.cancelled = true

    try {
      await oauthApi.cancelFlow(state.value.flowId)
    } finally {
      state.value.isAuthenticating = false
      state.value.flowId = null
    }
  }

  /**
   * Refresh OAuth token
   */
  const refreshToken = async (oauthConfig: OAuthConfig): Promise<OAuthConfig> => {
    const newConfig = await oauthApi.refreshToken(oauthConfig)
    state.value.config = newConfig
    return newConfig
  }

  /**
   * Check OAuth status
   */
  const checkStatus = async (oauthConfig: OAuthConfig): Promise<OAuthStatus> => {
    return await oauthApi.checkStatus(oauthConfig)
  }

  /**
   * Reset state
   */
  const reset = (): void => {
    state.value = {
      isAuthenticating: false,
      flowId: null,
      cancelled: false,
      config: null,
    }
  }

  return {
    isAuthenticating,
    config,
    startAuthorization,
    cancelAuthorization,
    refreshToken,
    checkStatus,
    reset,
  }
}
