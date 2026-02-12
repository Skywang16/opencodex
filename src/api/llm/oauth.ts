import type { OAuthConfig, OAuthFlowInfo, OAuthProvider, OAuthStatus } from '@/types/oauth'
import { invoke } from '@/utils/request'

export class OAuthApi {
  /**
   * Start OAuth flow
   */
  startFlow = async (provider: OAuthProvider): Promise<OAuthFlowInfo> => {
    return await invoke<OAuthFlowInfo>('start_oauth_flow', { provider })
  }

  /**
   * Wait for OAuth callback
   */
  waitForCallback = async (flowId: string, provider: OAuthProvider): Promise<OAuthConfig> => {
    return await invoke<OAuthConfig>('wait_oauth_callback', { flowId, provider })
  }

  /**
   * Cancel OAuth flow
   */
  cancelFlow = async (flowId: string): Promise<void> => {
    return await invoke<void>('cancel_oauth_flow', { flowId })
  }

  /**
   * Refresh OAuth token
   */
  refreshToken = async (oauthConfig: OAuthConfig): Promise<OAuthConfig> => {
    return await invoke<OAuthConfig>('refresh_oauth_token', { oauthConfig })
  }

  /**
   * Check OAuth status
   */
  checkStatus = async (oauthConfig: OAuthConfig): Promise<OAuthStatus> => {
    const status = await invoke<string>('check_oauth_status', { oauthConfig })
    return status as OAuthStatus
  }
}

export const oauthApi = new OAuthApi()
export default oauthApi
