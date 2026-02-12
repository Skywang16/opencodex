export interface ChannelCallbacks<T> {
  onMessage: (message: T) => void
  onError?: (error: unknown) => void
}

export interface ChannelSubscription {
  unsubscribe: () => Promise<void>
}

export interface ChannelOptions<T = unknown> {
  /** Custom cancel command name */
  cancelCommand?: string
  /** Function to determine if stream should be closed (for ReadableStream) */
  shouldClose?: (chunk: T) => boolean
}
