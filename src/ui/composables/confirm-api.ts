/**
 * Unified confirmation dialog API
 */

import enMessages from '@/i18n/locales/en.json'
import zhMessages from '@/i18n/locales/zh.json'
import { createApp, h, ref } from 'vue'
import { createI18n } from 'vue-i18n'
import XButton from '../components/Button.vue'
import XModal from '../components/Modal.vue'

export interface ConfirmConfig {
  /** Confirmation message content */
  message: string
  /** Dialog title */
  title?: string
  /** Confirm button text */
  confirmText?: string
  /** Cancel button text */
  cancelText?: string
  /** Dialog type, affects styling */
  type?: 'info' | 'warning' | 'danger'
  /** Whether to show cancel button */
  showCancelButton?: boolean
}

/**
 * Show confirmation dialog
 */
export const confirm = (config: string | ConfirmConfig): Promise<boolean> => {
  return new Promise(resolve => {
    // Normalize configuration
    const normalizedConfig: Required<ConfirmConfig> = {
      message: typeof config === 'string' ? config : config.message,
      title: typeof config === 'string' ? '' : config.title || '',
      confirmText: typeof config === 'string' ? '' : config.confirmText || '',
      cancelText: typeof config === 'string' ? '' : config.cancelText || '',
      type: typeof config === 'string' ? 'info' : config.type || 'info',
      showCancelButton: typeof config === 'string' ? true : (config.showCancelButton ?? true),
    }

    // Create container element
    const container = document.createElement('div')
    document.body.appendChild(container)

    // Reactive state
    const visible = ref(true)

    // Cleanup state flag to prevent duplicate cleanup
    let isCleanedUp = false

    // Safe cleanup function
    const cleanup = () => {
      if (isCleanedUp) {
        return
      }

      isCleanedUp = true

      try {
        // Safely unmount Vue app
        if (app) {
          app.unmount()
        }
      } catch (error) {
        console.warn('Failed to unmount confirm dialog app:', error)
      }

      try {
        // Safely remove DOM element
        if (container && container.parentNode === document.body) {
          document.body.removeChild(container)
        }
      } catch (error) {
        console.warn('Failed to remove confirm dialog container:', error)
      }
    }

    // Result processing state to prevent duplicate processing
    let isResolved = false

    // Unified result handling function
    const handleResult = (result: boolean) => {
      if (isResolved) {
        return
      }

      isResolved = true
      visible.value = false

      setTimeout(() => {
        cleanup()
        resolve(result)
      }, 150) // Wait for animation to complete
    }

    // Handle confirm
    const handleConfirm = () => {
      handleResult(true)
    }

    // Handle cancel
    const handleCancel = () => {
      handleResult(false)
    }

    // Handle close
    const handleClose = () => {
      handleResult(false)
    }

    // Create i18n instance
    const i18n = createI18n({
      legacy: false,
      locale: 'zh',
      fallbackLocale: 'en',
      messages: {
        zh: zhMessages,
        en: enMessages,
      },
    })

    // Create Vue app instance
    const app = createApp({
      setup() {
        return () =>
          h(
            XModal,
            {
              visible: visible.value,
              'onUpdate:visible': (newVisible: boolean) => {
                visible.value = newVisible
                // Don't call handleClose here to avoid conflicts with other event handlers
                // Let specific event handlers (onCancel, onClose, etc.) handle the result
              },
              title: normalizedConfig.title,
              size: 'small',
              showFooter: true,
              showCancelButton: normalizedConfig.showCancelButton,
              showConfirmButton: true,
              cancelText: normalizedConfig.cancelText,
              confirmText: normalizedConfig.confirmText,
              maskClosable: true,
              closable: true,
              onConfirm: handleConfirm,
              onCancel: handleCancel,
              onClose: handleClose,
              modalClass: `confirm-modal confirm-modal--${normalizedConfig.type}`,
              confirmButtonClass: normalizedConfig.type === 'danger' ? 'danger' : '',
            },
            {
              default: () =>
                h(
                  'div',
                  {
                    class: 'confirm-content',
                  },
                  [
                    // Icon
                    normalizedConfig.type === 'warning' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--warning',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('path', {
                                d: 'm21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z',
                              }),
                              h('path', { d: 'M12 9v4' }),
                              h('path', { d: 'm12 17 .01 0' }),
                            ]
                          ),
                        ]
                      ),

                    normalizedConfig.type === 'danger' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--danger',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('circle', { cx: '12', cy: '12', r: '10' }),
                              h('path', { d: 'M15 9l-6 6' }),
                              h('path', { d: 'M9 9l6 6' }),
                            ]
                          ),
                        ]
                      ),

                    normalizedConfig.type === 'info' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--info',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('circle', { cx: '12', cy: '12', r: '10' }),
                              h('path', { d: 'M12 16v-4' }),
                              h('path', { d: 'M12 8h.01' }),
                            ]
                          ),
                        ]
                      ),

                    // Message content
                    h(
                      'div',
                      {
                        class: 'confirm-message',
                      },
                      normalizedConfig.message
                    ),
                  ]
                ),
            }
          )
      },
    })

    // Install i18n plugin
    app.use(i18n)

    // Register x-button component
    app.component('x-button', XButton)

    // Add global styles
    const style = document.createElement('style')
    style.textContent = `
      .confirm-content {
        display: flex;
        align-items: center;
        gap: 16px;
        font-size: 13px;
        line-height: 1.4;
        min-height: 24px;
      }

      .confirm-icon {
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .confirm-icon--warning {
        color: #f9c74f;
      }

      .confirm-icon--danger {
        color: #f85149;
      }

      .confirm-icon--info {
        color: #58a6ff;
      }

      .confirm-message {
        color: var(--text-100);
        flex: 1;
        font-size: 13px;
        line-height: 1.4;
      }

      .confirm-modal .modal-button-primary.danger {
        background-color: #da3633;
        border-color: #da3633;
      }

      .confirm-modal .modal-button-primary.danger:hover:not(:disabled) {
        background-color: #e5484d;
        border-color: #e5484d;
      }

      .confirm-modal .modal-button-primary.danger:active:not(:disabled) {
        background-color: #cd2b31;
      }
    `
    document.head.appendChild(style)

    // Mount app
    app.mount(container)
  })
}

/**
 * Convenience method: show warning confirmation dialog
 */
export const confirmWarning = (message: string, title = 'Warning'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'warning',
  })
}

/**
 * Convenience method: show danger confirmation dialog
 */
export const confirmDanger = (message: string, title = 'Dangerous Operation'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'danger',
  })
}

/**
 * Convenience method: show info confirmation dialog
 */
export const confirmInfo = (message: string, title = 'Confirm'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'info',
  })
}
