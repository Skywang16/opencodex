import type { AIModelConfig, OAuthTokenResult } from '@/types/domain/ai'
import type { OAuthFlowInfo, OAuthProvider, OAuthStatus } from '@/types/oauth'
import { invoke } from '@/utils/request'

export class OAuthApi {
  startFlow = async (provider: OAuthProvider): Promise<OAuthFlowInfo> => {
    return await invoke<OAuthFlowInfo>('start_oauth_flow', { provider })
  }

  /** Returns token bundle — caller merges into AIModelConfig when saving */
  waitForCallback = async (flowId: string, provider: OAuthProvider): Promise<OAuthTokenResult> => {
    return await invoke<OAuthTokenResult>('wait_oauth_callback', { flowId, provider })
  }

  cancelFlow = async (flowId: string): Promise<void> => {
    return await invoke<void>('cancel_oauth_flow', { flowId })
  }

  refreshToken = async (model: AIModelConfig): Promise<AIModelConfig> => {
    return await invoke<AIModelConfig>('refresh_oauth_token', { model })
  }

  checkStatus = async (model: AIModelConfig): Promise<OAuthStatus> => {
    const status = await invoke<string>('check_oauth_status', { model })
    return status as OAuthStatus
  }
}

export const oauthApi = new OAuthApi()
export default oauthApi
