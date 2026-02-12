<script setup lang="ts">
  import { getCurrentLocale, setLocale } from '@/i18n'
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()

  const currentLocale = computed(() => getCurrentLocale())

  const languages = [
    { code: 'en-US', name: 'English', nativeName: 'English' },
    { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '简体中文' },
  ]

  const handleLanguageChange = async (code: string) => {
    await setLocale(code)
  }
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('settings.language.title') }}</h1>

    <div class="settings-group">
      <h2 class="group-title">{{ t('settings.language.interface_language') }}</h2>

      <div class="settings-card">
        <div
          v-for="lang in languages"
          :key="lang.code"
          class="settings-row clickable"
          :class="{ active: currentLocale === lang.code }"
          @click="handleLanguageChange(lang.code)"
        >
          <div class="row-info">
            <span class="row-label">{{ lang.nativeName }}</span>
            <span class="row-description">{{ lang.name }}</span>
          </div>
          <div class="row-control">
            <div class="radio-indicator" :class="{ checked: currentLocale === lang.code }">
              <svg
                v-if="currentLocale === lang.code"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="3"
                class="check-icon"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </div>
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

  .settings-row.clickable {
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .settings-row.clickable:hover {
    background: var(--color-hover);
  }

  .settings-row.active {
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
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

  .radio-indicator {
    width: 22px;
    height: 22px;
    border: 2px solid var(--border-300);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .radio-indicator.checked {
    border-color: var(--color-primary);
    background: var(--color-primary);
  }

  .check-icon {
    width: 14px;
    height: 14px;
    color: var(--bg-100);
  }
</style>
