import oauthApi from '@/api/llm/oauth'
import type { OAuthTokenResult } from '@/types/domain/ai'
import type { OAuthProvider } from '@/types/oauth'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

export const useOAuth = () => {
  const isAuthenticating = ref(false)
  const flowId = ref<string | null>(null)
  const cancelled = ref(false)

  /**
   * Start OAuth authorization flow.
   * Returns token bundle — caller merges into AIModelConfig when saving.
   */
  const startAuthorization = async (provider: OAuthProvider): Promise<OAuthTokenResult | null> => {
    isAuthenticating.value = true
    cancelled.value = false

    try {
      const flowInfo = await oauthApi.startFlow(provider)
      flowId.value = flowInfo.flowId

      await openUrl(flowInfo.authorizeUrl)

      const credential = await oauthApi.waitForCallback(flowInfo.flowId, provider)
      return credential
    } catch (err) {
      if (cancelled.value) return null
      throw err
    } finally {
      isAuthenticating.value = false
      flowId.value = null
    }
  }

  const cancelAuthorization = async (): Promise<void> => {
    if (!flowId.value) return
    cancelled.value = true
    try {
      await oauthApi.cancelFlow(flowId.value)
    } finally {
      isAuthenticating.value = false
      flowId.value = null
    }
  }

  return {
    isAuthenticating: computed(() => isAuthenticating.value),
    startAuthorization,
    cancelAuthorization,
  }
}
