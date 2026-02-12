<script setup lang="ts">
  import { onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAISettingsStore } from './store'
  import AIModelConfig from './components/AIModelConfig.vue'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()

  const init = async () => {
    if (!aiSettingsStore.isInitialized && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  }

  onMounted(async () => {
    await init()
  })

  defineExpose({ init })
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('settings.ai.title') }}</h1>
    <AIModelConfig />
  </div>
</template>

<style scoped>
  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .section-title {
    font-size: 24px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 8px 0;
  }
</style>
