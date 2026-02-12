<script setup lang="ts">
  import { XSwitch, createMessage } from '@/ui'
  import { disable as disableAutostart, enable as enableAutostart, isEnabled } from '@tauri-apps/plugin-autostart'
  import { debounce } from 'lodash-es'
  import { onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()

  const autoStartEnabled = ref(false)

  const loadAutoStartStatus = async () => {
    autoStartEnabled.value = await isEnabled()
  }

  const handleAutoStartToggle = debounce(async (enabled: boolean) => {
    if (enabled) {
      await enableAutostart()
      createMessage.success(t('settings.general.autostart_enabled'))
    } else {
      await disableAutostart()
      createMessage.success(t('settings.general.autostart_disabled'))
    }
    autoStartEnabled.value = enabled
  }, 300)

  const init = async () => {
    await loadAutoStartStatus()
  }

  onMounted(async () => {
    await init()
  })

  defineExpose({ init })
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('settings.general.title') }}</h1>

    <!-- Startup Group -->
    <div class="settings-group">
      <h2 class="group-title">{{ t('settings.general.startup_title') }}</h2>

      <div class="settings-card">
        <div class="settings-row">
          <div class="row-info">
            <span class="row-label">{{ t('settings.general.autostart_title') }}</span>
            <span class="row-description">{{ t('settings.general.autostart_description') }}</span>
          </div>
          <div class="row-control">
            <XSwitch :model-value="autoStartEnabled" @update:model-value="handleAutoStartToggle" />
          </div>
        </div>
      </div>
    </div>

    <!-- About Group -->
    <div class="settings-group">
      <h2 class="group-title">{{ t('settings.general.about') }}</h2>

      <div class="settings-card">
        <div class="settings-row">
          <div class="row-info">
            <span class="row-label">{{ t('settings.general.version') }}</span>
            <span class="row-description">{{ t('settings.general.version_description') }}</span>
          </div>
          <div class="row-control">
            <span class="version-text">v0.1.0</span>
          </div>
        </div>
      </div>
    </div>
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

  .settings-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-400);
    margin: 0;
    padding-left: 4px;
  }

  .settings-card {
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    overflow: hidden;
  }

  .settings-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    min-height: 60px;
  }

  .settings-row:not(:last-child) {
    border-bottom: 1px solid var(--border-100);
  }

  .row-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
    padding-right: 16px;
  }

  .row-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-100);
  }

  .row-description {
    font-size: 13px;
    color: var(--text-400);
    line-height: 1.4;
  }

  .row-control {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .version-text {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    font-family: var(--font-family-mono);
  }
</style>
