import { invoke } from '@/utils/request'
import { listen } from '@tauri-apps/api/event'
import { createI18n } from 'vue-i18n'
import en from './locales/en.json'
import zh from './locales/zh.json'

export type SupportedLanguage = 'zh-CN' | 'en-US'

export type MessageLanguages = keyof typeof zh

const messages = {
  'zh-CN': zh,
  'en-US': en,
}

export const i18n = createI18n({
  legacy: false,
  locale: 'en-US', // Set default value first to avoid initialization issues
  fallbackLocale: 'en-US',
  messages,
  globalInjection: true,
  silentFallbackWarn: true,
  silentTranslationWarn: true,
})

const getPersistedLanguage = async (): Promise<SupportedLanguage | null> => {
  const lang = await invoke<string>('language_get_app_language').catch(() => null)
  return lang === 'zh-CN' || lang === 'en-US' ? lang : null
}

const persistLanguage = async (language: SupportedLanguage): Promise<void> => {
  await invoke<void>('language_set_app_language', { language })
}

// Asynchronously initialize language settings
export const initLocale = async () => {
  let locale: SupportedLanguage | null = null
  try {
    locale = await getPersistedLanguage()
  } catch {
    locale = null
  }

  if (!locale) {
    const browserLang = navigator?.language?.toLowerCase() || ''
    locale = browserLang.startsWith('zh') ? 'zh-CN' : 'en-US'
  }

  i18n.global.locale.value = locale

  // Listen to backend language change events to keep display in sync
  await listen<string>('language-changed', event => {
    const next = event.payload
    if (next === 'zh-CN' || next === 'en-US') {
      i18n.global.locale.value = next
    }
  }).catch(error => {
    console.warn('Failed to setup language listener:', error)
  })
}

// Language switching function
export const setLocale = async (locale: string) => {
  // Validate locale parameter
  if (!locale || typeof locale !== 'string') {
    console.error('Invalid locale type:', typeof locale, locale)
    return
  }

  // Ensure locale is a supported language
  if (locale !== 'zh-CN' && locale !== 'en-US') {
    console.error('Unsupported locale:', locale)
    return
  }

  try {
    i18n.global.locale.value = locale as 'zh-CN' | 'en-US'

    // Notify backend language manager (write config and broadcast event)
    await persistLanguage(locale as SupportedLanguage)
  } catch (error) {
    console.error('Failed to save locale to backend:', error)
  }
}

export const getCurrentLocale = () => {
  return i18n.global.locale.value
}
