import { createApp, h } from 'vue'
import XMessage from '../components/Message.vue'

// Message type
export type MessageType = 'success' | 'error' | 'warning' | 'info'

// Message configuration interface
export interface MessageConfig {
  message: string
  type?: MessageType
  duration?: number
  closable?: boolean
  onClose?: () => void
  id?: string
}

// Message instance interface
export interface MessageInstance {
  id: string
  close: () => void
  update: (config: Partial<MessageConfig>) => void
}

// Message queue management
class MessageManager {
  private instances: Map<string, MessageInstance> = new Map()
  private container: HTMLElement | null = null
  private zIndex = 1000

  // Get or create container
  private getContainer(): HTMLElement {
    if (!this.container) {
      this.container = document.createElement('div')
      this.container.className = 'x-message-container'
      this.container.style.cssText = `
        position: fixed;
        top: 20px;
        left: 50%;
        transform: translateX(-50%);
        z-index: ${this.zIndex};
        pointer-events: none;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
      `
      document.body.appendChild(this.container)
    }
    return this.container
  }

  // Create message instance
  create = (config: MessageConfig): MessageInstance => {
    const id = config.id || `message_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
    const container = this.getContainer()

    // Default configuration
    const defaultConfig = {
      duration: 3000,
      maxCount: 5,
      placement: 'top-right',
    }

    // Check maximum count limit
    if (this.instances.size >= defaultConfig.maxCount) {
      // Remove oldest message
      const firstId = this.instances.keys().next().value
      if (firstId) {
        this.instances.get(firstId)?.close()
      }
    }

    // Create message element container
    const messageElement = document.createElement('div')
    messageElement.style.pointerEvents = 'auto'
    container.appendChild(messageElement)

    // Create Vue app instance
    const app = createApp({
      render() {
        return h(XMessage, {
          visible: true,
          message: config.message,
          type: config.type || 'info',
          duration: config.duration ?? defaultConfig.duration,
          onClose: () => {
            instance.close()
          },
        })
      },
    })

    // Mount app
    app.mount(messageElement)

    // Create instance object
    const instance: MessageInstance = {
      id,
      close: () => {
        // Remove DOM element
        if (messageElement.parentNode) {
          messageElement.parentNode.removeChild(messageElement)
        }
        // Unmount Vue app
        app.unmount()
        // Remove from instance map
        this.instances.delete(id)
        // Call close callback
        config.onClose?.()

        // If no messages remain, remove container
        if (this.instances.size === 0 && this.container) {
          document.body.removeChild(this.container)
          this.container = null
        }
      },
      update: (newConfig: Partial<MessageConfig>) => {
        // Update configuration (can implement more complex update logic here if needed)
        Object.assign(config, newConfig)
      },
    }

    // Store instance
    this.instances.set(id, instance)

    return instance
  }

  // Close all messages
  closeAll = (): void => {
    this.instances.forEach(instance => instance.close())
  }

  // Close message by ID
  close = (id: string): void => {
    this.instances.get(id)?.close()
  }
}

// Global message manager instance
const messageManager = new MessageManager()

// Main function to create message
export const createMessage = (config: string | MessageConfig): MessageInstance => {
  const messageConfig: MessageConfig = typeof config === 'string' ? { message: config } : config

  return messageManager.create(messageConfig)
}

// Convenience methods
createMessage.success = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'success', duration })
}

createMessage.error = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'error', duration })
}

createMessage.warning = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'warning', duration })
}

createMessage.info = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'info', duration })
}

// Close all messages
createMessage.closeAll = (): void => {
  messageManager.closeAll()
}

// Close message by ID
createMessage.close = (id: string): void => {
  messageManager.close(id)
}

// Default export
export default createMessage
