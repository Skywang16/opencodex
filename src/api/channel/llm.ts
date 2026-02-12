import { channelApi } from './index'
import { invoke } from '@/utils/request'

type LLMStreamChunk = { type: string; [key: string]: unknown }

/**
 * LLM dedicated Channel API
 */
class LLMChannelApi {
  /**
   * Create LLM streaming call
   */
  createStream = (request: Record<string, unknown>): ReadableStream<LLMStreamChunk> => {
    return channelApi.createStream<LLMStreamChunk>(
      'llm_call_stream',
      { request },
      {
        cancelCommand: 'llm_cancel_stream',
        shouldClose: (chunk: LLMStreamChunk) => {
          return chunk.type === 'finish' || chunk.type === 'error'
        },
      }
    )
  }

  /**
   * Cancel streaming call
   */
  cancelStream = async (requestId = 'current'): Promise<void> => {
    await invoke('llm_cancel_stream', { requestId })
  }
}

export const llmChannelApi = new LLMChannelApi()
