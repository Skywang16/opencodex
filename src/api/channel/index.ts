import { Channel, invoke } from '@tauri-apps/api/core'
import type { ChannelCallbacks, ChannelSubscription, ChannelOptions } from './types'

/**
 * Unified Channel API wrapper
 * Provides a unified interface similar to invoke wrapper, supporting different types of streaming data subscriptions
 */
class ChannelApi {
  /**
   * Generic subscription method
   */
  subscribe<T>(
    command: string,
    payload: Record<string, unknown>,
    callbacks: ChannelCallbacks<T>,
    options?: ChannelOptions<T>
  ): ChannelSubscription {
    const channel = new Channel<T>()
    channel.onmessage = callbacks.onMessage

    // Handle error callback
    if (callbacks.onError && 'onerror' in channel) {
      ;(channel as Channel<T> & { onerror?: (error: unknown) => void }).onerror = callbacks.onError
    }

    // Trigger backend subscription command
    invoke(command, { ...payload, channel }).catch(err => {
      if (callbacks.onError) callbacks.onError(err)
      else console.warn(`[channelApi] invoke ${command} error:`, err)
    })

    return {
      unsubscribe: async () => {
        try {
          const cancelCommand = options?.cancelCommand || `${command}_cancel`
          await invoke(cancelCommand, payload)
        } catch (err) {
          console.warn(`[channelApi] cancel ${command} error:`, err)
        }
      },
    }
  }

  /**
   * Create streaming ReadableStream (for scenarios like LLM that require ReadableStream)
   */
  createStream<T>(command: string, payload: Record<string, unknown>, options?: ChannelOptions<T>): ReadableStream<T> {
    const channel = new Channel<T>()
    let isStreamClosed = false

    return new ReadableStream({
      start(controller) {
        // Bind message and error handlers before calling backend to avoid early event loss
        channel.onmessage = (chunk: T) => {
          if (isStreamClosed) return

          try {
            controller.enqueue(chunk)

            // Check if stream should be closed
            if (options?.shouldClose?.(chunk)) {
              isStreamClosed = true
              controller.close()
            }
          } catch (error) {
            if (!isStreamClosed) {
              isStreamClosed = true
              controller.error(error)
            }
          }
        }

        // Handle Channel errors
        if ('onerror' in channel) {
          ;(channel as Channel<T> & { onerror?: (error: unknown) => void }).onerror = (error: unknown) => {
            if (!isStreamClosed) {
              isStreamClosed = true
              controller.error(new Error(`Channel error: ${error}`))
            }
          }
        }

        // Start backend command after binding callbacks to ensure early events are not missed
        invoke(command, { ...payload, channel }).catch(error => {
          console.error(`[channelApi] stream invoke ${command} error:`, error)
          if (!isStreamClosed) {
            isStreamClosed = true
            controller.error(error)
          }
        })
      },
      cancel() {
        isStreamClosed = true
        // Cancel backend command
        const cancelCommand = options?.cancelCommand || `${command}_cancel`
        invoke(cancelCommand, payload).catch(console.warn)
      },
    })
  }
}

export const channelApi = new ChannelApi()

// Export specific Channel API instance, similar to invoke usage
export { channelApi as channel }

// Export types
export * from './types'
