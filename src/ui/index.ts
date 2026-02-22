import type { App, Plugin } from 'vue'

import './styles/index.css'

import { XButton, XFormGroup, XInput, XMessage, XModal, XSearchInput, XSelect, XSwitch, XTextarea } from './components'

import {
  confirm,
  confirmDanger,
  confirmInfo,
  confirmWarning,
  createMessage,
  type ConfirmConfig,
  type MessageConfig,
  type MessageInstance,
} from './composables'

import { createPopover, showContextMenu, showPopoverAt } from './composables/popover-api'

// Main component exports (recommended usage)
export { XButton, XFormGroup, XInput, XMessage, XModal, XSearchInput, XSelect, XSwitch, XTextarea }

// System-level menu API
export { createPopover, showContextMenu, showPopoverAt }

// Message API
export { createMessage }
export type { MessageConfig, MessageInstance }

// Confirmation dialog API
export { confirm, confirmDanger, confirmInfo, confirmWarning }
export type { ConfirmConfig }

// Message API convenience methods (already defined on createMessage)
// createMessage.success(message, duration?)
// createMessage.error(message, duration?)
// createMessage.warning(message, duration?)
// createMessage.info(message, duration?)
// createMessage.closeAll()
// createMessage.close(id)

// Global configuration interface
export interface XUIGlobalConfig {
  // Global size
  size?: 'small' | 'medium' | 'large'
  // Global theme
  theme?: 'light' | 'dark'
  // Internationalization language
  locale?: string
  // Global z-index base
  zIndex?: number
  // Message component configuration
  message?: {
    duration?: number
    maxCount?: number
    placement?: 'top' | 'top-left' | 'top-right' | 'bottom' | 'bottom-left' | 'bottom-right'
  }
}

// Default configuration
const defaultConfig: Required<XUIGlobalConfig> = {
  size: 'medium',
  theme: 'light',
  locale: 'zh-CN',
  zIndex: 1000,
  message: {
    duration: 3000,
    maxCount: 5,
    placement: 'top-right',
  },
}

// Global configuration storage
let globalConfig: Required<XUIGlobalConfig> = { ...defaultConfig }

// Configuration management functions
export const getGlobalConfig = (): Required<XUIGlobalConfig> => globalConfig
export const setGlobalConfig = (config: Partial<XUIGlobalConfig>): void => {
  globalConfig = { ...globalConfig, ...config }
}

// Installation function
const install = (app: App, options: Partial<XUIGlobalConfig> = {}): void => {
  // Set global configuration
  setGlobalConfig(options)

  // Register all components - explicitly specify component names
  app.component('XButton', XButton)
  app.component('x-button', XButton)

  app.component('XMessage', XMessage)
  app.component('x-message', XMessage)

  app.component('XModal', XModal)
  app.component('x-modal', XModal)

  app.component('XSearchInput', XSearchInput)
  app.component('x-search-input', XSearchInput)

  app.component('XSelect', XSelect)
  app.component('x-select', XSelect)

  app.component('XSwitch', XSwitch)
  app.component('x-switch', XSwitch)

  // Form components
  app.component('XFormGroup', XFormGroup)
  app.component('x-form-group', XFormGroup)

  app.component('XInput', XInput)
  app.component('x-input', XInput)

  app.component('XTextarea', XTextarea)
  app.component('x-textarea', XTextarea)

  // Mount global methods
  app.config.globalProperties.$message = createMessage
  app.provide('xui-config', globalConfig)
}

// Component library plugin type
type XUIPlugin = Plugin & {
  version: string
  install: (app: App, options?: Partial<XUIGlobalConfig>) => void
}

// Component library plugin
const XUI: XUIPlugin = {
  install,
  version: '1.0.0',
}

// Installation function export
export { install }

// Default export (plugin)
export default XUI

export * from './types/index'

export type {
  ButtonEmits,
  ButtonProps,
  FormStatus,
  FormGroupProps,
  InputEmits,
  InputProps,
  MessageEmits,
  MessageProps,
  ModalEmits,
  ModalProps,
  Placement,
  SearchInputEmits,
  SearchInputProps,
  SelectEmits,
  SelectOption,
  SelectProps,
  Size,
  SwitchEmits,
  SwitchProps,
  TextareaEmits,
  TextareaProps,
  Theme,
} from './types/index'
