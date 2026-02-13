<script setup lang="ts">
  import type { AIModelCreateInput, AIModelTestConnectionInput } from '@/api/ai/types'
  import type { AIModelConfig, AIProvider } from '@/types'
  import { AuthType, OAuthProvider, type OAuthConfig } from '@/types/oauth'

  import { aiApi } from '@/api'
  import { useLLMRegistry, type ModelOption } from '@/composables/useLLMRegistry'
  import { useOAuth } from '@/composables/useOAuth'
  import { confirmDanger } from '@/ui'
  import { computed, onMounted, reactive, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAISettingsStore } from '../store'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()
  const { providers, providerOptions, getChatModelOptions, getModelInfo, loadProviders } = useLLMRegistry()

  const models = computed(() => aiSettingsStore.chatModels)
  const loading = computed(() => aiSettingsStore.isLoading)

  const editingId = ref<string | null>(null)
  const isTesting = ref(false)
  const isSaving = ref(false)
  const showAdvancedOptions = ref(false)
  const showAddForm = ref(false)
  const showApiKey = ref(false)

  const { isAuthenticating, startAuthorization: startOAuth, cancelAuthorization: cancelOAuth } = useOAuth()

  const formData = reactive({
    authType: 'apikey' as 'apikey' | 'oauth',
    provider: '' as string,
    apiUrl: '',
    apiKey: '',
    model: '',
    useCustomBaseUrl: false,
    useCustomModel: false,
    oauthProvider: '' as string,
    oauthConfig: undefined as OAuthConfig | undefined,
    options: {
      maxContextTokens: 128000,
      temperature: 0.5,
      timeoutSeconds: 300,
      maxTokens: -1,
      enableDeepThinking: false,
    },
  })

  const availableModels = computed<ModelOption[]>(() => {
    if (formData.authType === 'oauth') {
      return [
        { value: 'gpt-4o', label: 'GPT-4o', reasoning: false, toolCall: true, attachment: true },
        { value: 'gpt-4o-mini', label: 'GPT-4o Mini', reasoning: false, toolCall: true, attachment: true },
        { value: 'o1', label: 'o1', reasoning: true, toolCall: false, attachment: false },
        { value: 'o1-mini', label: 'o1 Mini', reasoning: true, toolCall: false, attachment: false },
      ]
    }
    return getChatModelOptions(formData.provider)
  })

  const providerInfo = computed(() => {
    if (formData.authType === 'oauth') return null
    return providers.value.find(p => p.id === formData.provider) || null
  })

  const hasPresetModels = computed(() => availableModels.value.length > 0)

  // Get current model info for capability check
  const selectedModelInfo = computed(() => {
    if (!formData.provider || !formData.model) return null
    return getModelInfo(formData.provider, formData.model)
  })

  // Check if selected model supports reasoning (deep thinking)
  const supportsDeepThinking = computed(() => {
    // Custom model ID — user decides, always show the toggle
    if (formData.useCustomModel) return true
    // For OAuth models, check from availableModels
    if (formData.authType === 'oauth') {
      const model = availableModels.value.find(m => m.value === formData.model)
      return model?.reasoning ?? false
    }
    // For API key models, check from models.dev data
    return selectedModelInfo.value?.reasoning ?? false
  })

  onMounted(async () => {
    await Promise.all([aiSettingsStore.loadModels(), loadProviders()])
  })

  const resetForm = () => {
    // If authorizing, cancel authorization flow
    if (isAuthenticating.value) {
      cancelOAuth()
    }
    formData.authType = 'apikey'
    formData.provider = ''
    formData.apiUrl = ''
    formData.apiKey = ''
    formData.model = ''
    formData.useCustomBaseUrl = false
    formData.useCustomModel = false
    formData.oauthProvider = ''
    formData.oauthConfig = undefined
    formData.options = {
      maxContextTokens: 128000,
      temperature: 0.7,
      timeoutSeconds: 300,
      maxTokens: -1,
      enableDeepThinking: false,
    }
    showAdvancedOptions.value = false
    editingId.value = null
    showAddForm.value = false
  }

  const startAdding = () => {
    resetForm()
    showAddForm.value = true
  }

  const startEditing = (model: AIModelConfig) => {
    editingId.value = model.id
    showAddForm.value = true
    if (model.authType === AuthType.OAuth) {
      formData.authType = 'oauth'
      formData.oauthProvider = model.oauthConfig?.provider || OAuthProvider.OpenAiCodex
      formData.oauthConfig = model.oauthConfig
      formData.model = model.model
      formData.provider = ''
    } else {
      formData.authType = 'apikey'
      formData.provider = model.provider
      formData.apiUrl = model.apiUrl || ''
      formData.apiKey = model.apiKey || ''
      formData.model = model.model
      formData.useCustomBaseUrl = model.useCustomBaseUrl || false
      formData.useCustomModel = !getChatModelOptions(model.provider).some(m => m.value === model.model)
      formData.oauthProvider = ''
    }
    formData.options = {
      maxContextTokens: model.options?.maxContextTokens ?? 128000,
      temperature: model.options?.temperature ?? 0.5,
      timeoutSeconds: model.options?.timeoutSeconds ?? 300,
      maxTokens: model.options?.maxTokens ?? -1,
      enableDeepThinking: model.options?.enableDeepThinking ?? false,
    }
  }

  const switchAuthType = (type: 'apikey' | 'oauth') => {
    formData.authType = type
    formData.model = ''
    if (type === 'oauth') {
      formData.provider = ''
      formData.apiKey = ''
      formData.apiUrl = ''
      formData.oauthProvider = OAuthProvider.OpenAiCodex
      formData.model = 'gpt-4o'
    } else {
      formData.oauthConfig = undefined
      formData.oauthProvider = ''
    }
  }

  const handleProviderChange = (value: string) => {
    formData.provider = value
    const info = providerInfo.value
    if (info) {
      formData.apiUrl = info.apiUrl || ''
      const models = getChatModelOptions(value)
      formData.model = models.length > 0 ? models[0].value : ''
      // Auto-set deep thinking based on first model's capability
      if (models.length > 0) {
        formData.options.enableDeepThinking = models[0].reasoning
      }
    }
    formData.useCustomBaseUrl = false
    formData.useCustomModel = false
  }

  const handleCustomUrlToggle = () => {
    formData.apiUrl = formData.useCustomBaseUrl ? '' : providerInfo.value?.apiUrl || ''
  }

  const handleStartAuth = async () => {
    const provider = (formData.oauthProvider as OAuthProvider) || OAuthProvider.OpenAiCodex
    const config = await startOAuth(provider)
    if (config) {
      formData.oauthConfig = config
    }
  }

  const testConnection = async () => {
    if (formData.authType === 'oauth' || !formData.provider || !formData.model || !formData.apiKey) return
    isTesting.value = true
    try {
      await aiApi.testConnectionWithConfig({
        provider: formData.provider as AIProvider,
        authType: AuthType.ApiKey,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        modelType: 'chat',
        options: formData.options,
      } as AIModelTestConnectionInput)
    } finally {
      isTesting.value = false
    }
  }

  const saveModel = async () => {
    if (formData.authType === 'oauth') {
      if (!formData.oauthConfig || !formData.model || !formData.oauthProvider) return
    } else {
      if (!formData.provider || !formData.model || !formData.apiKey) return
    }

    isSaving.value = true
    try {
      const modelData =
        formData.authType === 'oauth'
          ? {
              provider: 'openai_compatible' as AIProvider,
              authType: AuthType.OAuth,
              apiUrl: '',
              apiKey: '',
              model: formData.model,
              modelType: 'chat' as const,
              options: formData.options,
              oauthConfig: formData.oauthConfig,
              useCustomBaseUrl: false,
            }
          : {
              provider: formData.provider as AIProvider,
              authType: AuthType.ApiKey,
              apiUrl: formData.apiUrl,
              apiKey: formData.apiKey,
              model: formData.model,
              modelType: 'chat' as const,
              options: formData.options,
              oauthConfig: undefined,
              useCustomBaseUrl: formData.useCustomBaseUrl,
            }

      if (editingId.value) {
        await aiSettingsStore.updateModel(editingId.value, modelData)
      } else {
        await aiSettingsStore.addModel(modelData as AIModelCreateInput)
      }
      resetForm()
    } finally {
      isSaving.value = false
    }
  }

  const deleteModel = async (modelId: string) => {
    const confirmed = await confirmDanger(
      t('ai_model.delete_confirm'),
      t('ai_model.delete_confirm_text') || 'Delete Model'
    )
    if (!confirmed) return
    await aiSettingsStore.removeModel(modelId)
  }

  const isFormValid = computed(() => {
    if (formData.authType === 'oauth') {
      return !!formData.oauthConfig && !!formData.model && !!formData.oauthProvider
    }
    return !!formData.provider && !!formData.model && !!formData.apiKey?.trim()
  })
</script>

<template>
  <div class="ai-model-config">
    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <template v-else>
      <!-- Configured Models List -->
      <div class="settings-group">
        <div class="group-header">
          <h2 class="group-title">{{ t('ai_model.configured_models') || 'Configured Models' }}</h2>
          <button v-if="!showAddForm && models.length > 0" class="add-btn" @click="startAdding">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            {{ t('ai_model.add_model') || 'Add Model' }}
          </button>
        </div>

        <!-- Empty State -->
        <div v-if="models.length === 0 && !showAddForm" class="settings-card empty-card">
          <div class="empty-state">
            <div class="empty-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path
                  d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z"
                />
              </svg>
            </div>
            <div class="empty-text">
              <span class="empty-title">{{ t('ai_model.no_models') || 'No AI models configured' }}</span>
              <span class="empty-desc">
                {{ t('ai_model.add_model_hint') || 'Add a model to start using AI features' }}
              </span>
            </div>
            <button class="add-first-btn" @click="startAdding">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
              {{ t('ai_model.add_first_model') || 'Add Your First Model' }}
            </button>
          </div>
        </div>

        <!-- Models List -->
        <div v-if="models.length > 0" class="models-list">
          <div v-for="model in models" :key="model.id" class="model-card">
            <div class="model-icon" :class="model.authType === AuthType.OAuth ? 'oauth' : 'apikey'">
              <svg v-if="model.authType === AuthType.OAuth" viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729z"
                />
              </svg>
              <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path
                  d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"
                />
              </svg>
            </div>
            <div class="model-info">
              <span class="model-name">{{ model.model }}</span>
              <div class="model-meta">
                <span class="provider-name">{{ model.provider }}</span>
                <span class="model-badge" :class="model.authType === AuthType.OAuth ? 'oauth' : 'apikey'">
                  {{ model.authType === AuthType.OAuth ? 'OAuth' : 'API Key' }}
                </span>
              </div>
            </div>
            <div class="model-actions">
              <button class="action-btn edit" @click="startEditing(model)" :title="t('common.edit')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                </svg>
              </button>
              <button class="action-btn delete" @click="deleteModel(model.id)" :title="t('common.delete')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  <line x1="10" y1="11" x2="10" y2="17" />
                  <line x1="14" y1="11" x2="14" y2="17" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Add/Edit Form -->
      <div v-if="showAddForm" class="settings-group">
        <h2 class="group-title">
          {{ editingId ? t('ai_model.edit_model') || 'Edit Model' : t('ai_model.add_model') || 'Add Model' }}
        </h2>

        <div class="settings-card form-card">
          <!-- Auth Type Segmented Control -->
          <div class="auth-segment-wrapper">
            <div class="auth-segment">
              <div class="segment-slider" :class="{ right: formData.authType === 'oauth' }"></div>
              <button
                class="segment-btn"
                :class="{ active: formData.authType === 'apikey' }"
                @click="switchAuthType('apikey')"
              >
                API Key
              </button>
              <button
                class="segment-btn"
                :class="{ active: formData.authType === 'oauth' }"
                @click="switchAuthType('oauth')"
              >
                {{ t('ai_model.subscription') || 'Subscription' }}
              </button>
            </div>
          </div>

          <!-- API Key Form -->
          <div v-if="formData.authType === 'apikey'" class="form-body">
            <div class="form-group">
              <label class="form-label">{{ t('ai_model.provider') }}</label>
              <x-select
                v-model="formData.provider"
                :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
                :placeholder="t('ai_model.select_provider')"
                @update:modelValue="handleProviderChange"
              />
            </div>

            <div v-if="formData.provider" class="form-group">
              <label class="form-label">{{ t('ai_model.api_key') }}</label>
              <div class="input-with-toggle">
                <input
                  v-model="formData.apiKey"
                  :type="showApiKey ? 'text' : 'password'"
                  class="form-input mono"
                  :placeholder="t('ai_model.api_key_placeholder')"
                />
                <button type="button" class="toggle-visibility-btn" @click="showApiKey = !showApiKey" tabindex="-1">
                  <svg v-if="showApiKey" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                    <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                    <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24" />
                    <line x1="1" y1="1" x2="23" y2="23" />
                  </svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </svg>
                </button>
              </div>
            </div>

            <div v-if="formData.provider && hasPresetModels" class="form-group inline">
              <label class="form-label">{{ t('ai_model.use_custom_model') || 'Custom Model ID' }}</label>
              <x-switch
                :modelValue="formData.useCustomModel"
                @update:modelValue="
                  (v: boolean) => {
                    formData.useCustomModel = v
                    formData.model = ''
                  }
                "
              />
            </div>

            <div v-if="formData.provider && hasPresetModels && !formData.useCustomModel" class="form-group">
              <label class="form-label">{{ t('ai_model.model') }}</label>
              <x-select v-model="formData.model" :options="availableModels" :placeholder="t('ai_model.select_model')" />
            </div>

            <div v-else-if="formData.provider" class="form-group">
              <label class="form-label">{{ t('ai_model.model_name') }}</label>
              <input
                v-model="formData.model"
                type="text"
                class="form-input"
                :placeholder="t('ai_model.model_name_placeholder')"
              />
            </div>

            <div v-if="formData.provider && hasPresetModels" class="form-group inline">
              <label class="form-label">{{ t('ai_model.use_custom_base_url') }}</label>
              <x-switch
                :modelValue="formData.useCustomBaseUrl"
                @update:modelValue="
                  (v: boolean) => {
                    formData.useCustomBaseUrl = v
                    handleCustomUrlToggle()
                  }
                "
              />
            </div>

            <div v-if="formData.provider && (formData.useCustomBaseUrl || !hasPresetModels)" class="form-group">
              <label class="form-label">{{ t('ai_model.api_url') }}</label>
              <input
                v-model="formData.apiUrl"
                type="url"
                class="form-input mono"
                :placeholder="t('ai_model.api_url_placeholder')"
              />
            </div>

            <!-- Responses API / Deep Thinking toggle -->
            <div v-if="formData.model && supportsDeepThinking" class="form-group inline">
              <div class="label-with-badge">
                <label class="form-label">{{ t('ai_model.responses_api') || 'Responses API' }}</label>
                <span class="feature-badge reasoning">{{ t('ai_model.reasoning_model') || 'Reasoning' }}</span>
              </div>
              <x-switch v-model="formData.options.enableDeepThinking" />
            </div>
          </div>

          <!-- OAuth Form -->
          <div v-else class="form-body">
            <div class="oauth-info">
              <div class="oauth-provider-card">
                <div class="oauth-provider-icon">
                  <svg viewBox="0 0 24 24" fill="currentColor">
                    <path
                      d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.8956zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z"
                    />
                  </svg>
                </div>
                <div class="oauth-provider-info">
                  <div class="oauth-provider-name">ChatGPT Plus/Pro</div>
                  <div class="oauth-provider-desc">
                    {{ t('ai_model.oauth_description') || 'Use your ChatGPT subscription' }}
                  </div>
                </div>
              </div>

              <div class="oauth-status-row">
                <div
                  class="oauth-status"
                  :class="{ authorized: formData.oauthConfig, authenticating: isAuthenticating }"
                >
                  <svg
                    v-if="formData.oauthConfig"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    class="status-icon success"
                  >
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                  <div v-else-if="isAuthenticating" class="status-spinner"></div>
                  <svg
                    v-else
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    class="status-icon"
                  >
                    <circle cx="12" cy="12" r="10" />
                  </svg>
                  <span>
                    {{
                      isAuthenticating
                        ? t('ai_model.authorizing') || 'Authorizing...'
                        : formData.oauthConfig
                          ? t('ai_model.authorized') || 'Authorized'
                          : t('ai_model.not_authorized') || 'Not authorized'
                    }}
                  </span>
                </div>
                <button
                  class="auth-btn"
                  :class="{ secondary: formData.oauthConfig }"
                  :disabled="isAuthenticating"
                  @click="handleStartAuth"
                >
                  {{
                    formData.oauthConfig
                      ? t('ai_model.reauthorize') || 'Re-authorize'
                      : t('ai_model.start_authorization') || 'Authorize'
                  }}
                </button>
              </div>
            </div>

            <div v-if="formData.oauthConfig" class="form-group">
              <label class="form-label">{{ t('ai_model.model') }}</label>
              <x-select v-model="formData.model" :options="availableModels" :placeholder="t('ai_model.select_model')" />
            </div>
          </div>

          <!-- Form Actions -->
          <div class="form-actions">
            <button class="cancel-btn" @click="resetForm">{{ t('common.cancel') }}</button>
            <button
              v-if="formData.authType === 'apikey'"
              class="secondary-btn"
              :disabled="!isFormValid || isTesting"
              @click="testConnection"
            >
              {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
            </button>
            <button class="primary-btn" :disabled="!isFormValid || isSaving" @click="saveModel">
              {{ editingId ? t('common.save') : t('common.add') }}
            </button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
  .ai-model-config {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .settings-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px;
  }

  .group-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-400);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings-card {
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    overflow: hidden;
  }

  /* Loading State */
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    color: var(--text-400);
    font-size: 13px;
  }

  /* Empty State */
  .empty-card {
    background: linear-gradient(135deg, var(--bg-200) 0%, var(--bg-300) 100%);
    border: 1px dashed var(--border-300);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 48px 24px;
    text-align: center;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      var(--color-primary) 0%,
      color-mix(in srgb, var(--color-primary) 70%, var(--color-primary)) 100%
    );
    border-radius: var(--border-radius-2xl);
    color: var(--bg-100);
    box-shadow: 0 8px 24px -4px color-mix(in srgb, var(--color-primary) 30%, transparent);
  }

  .empty-icon svg {
    width: 28px;
    height: 28px;
  }

  .empty-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-200);
  }

  .empty-desc {
    font-size: 13px;
    color: var(--text-400);
  }

  .add-first-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 600;
    color: white;
    background: var(--color-primary);
    border: none;
    border-radius: var(--border-radius-xl);
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px -2px color-mix(in srgb, var(--color-primary) 40%, transparent);
  }

  .add-first-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--color-primary) 50%, transparent);
  }

  .add-first-btn:active {
    transform: translateY(0);
  }

  .add-first-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Models List */
  .models-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .model-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    transition: all 0.2s ease;
  }

  .model-card:hover {
    border-color: var(--border-300);
    box-shadow: 0 2px 8px -2px var(--shadow-color, rgba(0, 0, 0, 0.1));
  }

  .model-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--border-radius-xl);
    flex-shrink: 0;
  }

  .model-icon.apikey {
    background: linear-gradient(135deg, var(--bg-400) 0%, var(--bg-300) 100%);
    color: var(--text-300);
  }

  .model-icon.oauth {
    background: linear-gradient(135deg, #10a37f 0%, #0d8a6a 100%);
    color: white;
  }

  .model-icon svg {
    width: 20px;
    height: 20px;
  }

  .model-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .model-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
    font-family: var(--font-family-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .model-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .provider-name {
    color: var(--text-400);
    text-transform: capitalize;
  }

  .model-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: var(--border-radius-sm);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .model-badge.apikey {
    color: var(--text-400);
    background: var(--bg-400);
  }

  .model-badge.oauth {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .model-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .model-card:hover .model-actions {
    opacity: 1;
  }

  .action-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .action-btn.delete:hover {
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    color: var(--color-error);
  }

  .action-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Add Button */
  .add-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-btn:hover {
    background: var(--bg-400);
    color: var(--text-200);
    border-color: var(--border-300);
  }

  .add-btn svg {
    width: 14px;
    height: 14px;
  }

  /* Form Card */
  .form-card {
    overflow: visible;
    box-shadow: 0 4px 24px -8px var(--shadow-color, rgba(0, 0, 0, 0.12));
  }

  /* Segmented Control */
  .auth-segment-wrapper {
    padding: 16px 20px 0;
  }

  .auth-segment {
    position: relative;
    display: flex;
    padding: 3px;
    background: var(--bg-300);
    border-radius: var(--border-radius-xl);
    max-width: 280px;
  }

  .segment-slider {
    position: absolute;
    top: 3px;
    left: 3px;
    width: calc(50% - 3px);
    height: calc(100% - 6px);
    background: var(--bg-100);
    border-radius: var(--border-radius-lg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .segment-slider.right {
    transform: translateX(100%);
  }

  .segment-btn {
    flex: 1;
    position: relative;
    z-index: 1;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-400);
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: color 0.2s ease;
  }

  .segment-btn:hover:not(.active) {
    color: var(--text-300);
  }

  .segment-btn.active {
    color: var(--text-100);
  }

  .form-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .form-group.inline {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--bg-300);
    border-radius: var(--border-radius-xl);
  }

  .form-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
  }

  .label-with-badge {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .feature-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: var(--border-radius-sm);
    color: var(--text-400);
    background: var(--bg-400);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .feature-badge.reasoning {
    color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .form-input {
    padding: 11px 14px;
    font-size: 14px;
    color: var(--text-100);
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    outline: none;
    transition: all 0.2s ease;
  }

  .form-input:hover {
    border-color: var(--border-300);
  }

  .form-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .form-input.mono {
    font-family: var(--font-family-mono);
    font-size: 13px;
  }

  .form-input::placeholder {
    color: var(--text-500);
  }

  .input-with-toggle {
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-with-toggle .form-input {
    width: 100%;
    padding-right: 40px;
  }

  .toggle-visibility-btn {
    position: absolute;
    right: 8px;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-500);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-visibility-btn:hover {
    color: var(--text-300);
    background: var(--bg-400);
  }

  .toggle-visibility-btn svg {
    width: 16px;
    height: 16px;
  }

  /* OAuth */
  .oauth-info {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .oauth-provider-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    background: linear-gradient(135deg, color-mix(in srgb, #10a37f 8%, var(--bg-300)) 0%, var(--bg-300) 100%);
    border: 1px solid color-mix(in srgb, #10a37f 20%, var(--border-200));
    border-radius: var(--border-radius-xl);
  }

  .oauth-provider-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--color-success) 0%, var(--color-success) 100%);
    border-radius: var(--border-radius-xl);
    color: var(--bg-100);
    box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--color-success) 30%, transparent);
  }

  .oauth-provider-icon svg {
    width: 26px;
    height: 26px;
  }

  .oauth-provider-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .oauth-provider-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-100);
  }

  .oauth-provider-desc {
    font-size: 12px;
    color: var(--text-400);
  }

  .oauth-status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    background: var(--bg-300);
    border-radius: var(--border-radius-xl);
  }

  .oauth-status {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-400);
  }

  .oauth-status.authorized {
    color: var(--color-success);
  }

  .status-icon {
    width: 20px;
    height: 20px;
  }

  .status-icon.success {
    color: var(--color-success);
  }

  .oauth-status.authenticating {
    color: var(--color-primary);
  }

  .status-spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border-300);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .auth-btn {
    padding: 9px 16px;
    font-size: 13px;
    font-weight: 600;
    color: var(--bg-100);
    background: var(--color-primary);
    border: none;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .auth-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--color-primary) 40%, transparent);
  }

  .auth-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .auth-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .auth-btn.secondary {
    color: var(--text-200);
    background: var(--bg-400);
  }

  .auth-btn.secondary:hover:not(:disabled) {
    background: var(--bg-500);
    box-shadow: none;
  }

  .form-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    background: var(--bg-300);
    border-top: 1px solid var(--border-100);
  }

  .cancel-btn {
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    background: transparent;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-btn:hover {
    background: var(--bg-400);
    color: var(--text-200);
  }

  .secondary-btn {
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .secondary-btn:hover:not(:disabled) {
    background: var(--bg-100);
    border-color: var(--border-300);
  }

  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .primary-btn {
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 600;
    color: var(--bg-100);
    background: var(--color-primary);
    border: none;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .primary-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px -2px color-mix(in srgb, var(--color-primary) 40%, transparent);
  }

  .primary-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
