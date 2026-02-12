<script setup lang="ts">
  import { useThemeStore } from '@/stores/theme'
  import type { ThemeOption } from '@/types/domain/theme'
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  const themeStore = useThemeStore()
  const { t } = useI18n()

  const getThemeColors = (option: ThemeOption) => {
    const ui = option.ui
    if (!ui) {
      const isDark = option.type === 'dark'
      return {
        bg: isDark ? '#1a1a1a' : '#ffffff',
        sidebar: isDark ? '#242424' : '#f5f5f5',
        card: isDark ? '#2d2d2d' : '#fafafa',
        line: isDark ? '#3d3d3d' : '#e8e8e8',
        text: isDark ? '#e0e0e0' : '#1a1a1a',
        primary: isDark ? '#11a8cd' : '#6f42c1',
      }
    }
    return {
      bg: ui.bg_200,
      sidebar: ui.bg_300,
      card: ui.bg_400,
      line: ui.bg_500,
      text: ui.text_200,
      primary: ui.primary,
    }
  }

  const init = async () => {
    await themeStore.initialize()
    const config = themeStore.themeConfig
    if (config) {
      if (config.lightTheme) selectedLightTheme.value = config.lightTheme
      if (config.darkTheme) selectedDarkTheme.value = config.darkTheme
    }
  }

  onMounted(async () => {
    await init()
  })

  defineExpose({ init })

  const selectedLightTheme = ref('light')
  const selectedDarkTheme = ref('dark')

  const currentMode = computed(() => {
    if (themeStore.isFollowingSystem) return 'system'
    // Check current theme type
    const currentThemeType = themeStore.currentTheme?.themeType
    if (currentThemeType === 'light') return 'light'
    if (currentThemeType === 'dark') return 'dark'
    return 'light' // Default
  })

  const themeOptionsCache = computed(() => {
    return themeStore.themeOptions.map((option: ThemeOption) => ({
      value: option.value,
      label: option.label,
      type: option.type,
      isCurrent: option.isCurrent,
      ui: option.ui,
    }))
  })

  const manualThemeOptions = computed(() => themeOptionsCache.value)

  const systemLightThemeOptions = computed(() => {
    return themeOptionsCache.value.filter(option => option.type === 'light' || option.type === 'auto')
  })

  const systemDarkThemeOptions = computed(() => {
    return themeOptionsCache.value.filter(option => option.type === 'dark' || option.type === 'auto')
  })

  watch(
    () => themeStore.themeConfig,
    config => {
      if (config) {
        if (config.lightTheme) selectedLightTheme.value = config.lightTheme
        if (config.darkTheme) selectedDarkTheme.value = config.darkTheme
      }
    },
    { immediate: true }
  )

  const handleModeChange = async (mode: 'light' | 'dark' | 'system') => {
    if (currentMode.value === mode) return

    if (mode === 'system') {
      await themeStore.enableFollowSystem(selectedLightTheme.value || 'light', selectedDarkTheme.value || 'dark')
    } else if (mode === 'light') {
      // Switch to manual mode and select light theme
      await themeStore.disableFollowSystem()
      await themeStore.switchToTheme(selectedLightTheme.value || 'light')
    } else if (mode === 'dark') {
      // Switch to manual mode and select dark theme
      await themeStore.disableFollowSystem()
      await themeStore.switchToTheme(selectedDarkTheme.value || 'dark')
    }
  }

  const handleThemeSelect = async (themeName: string) => {
    await themeStore.switchToTheme(themeName)
  }

  const handleLightThemeSelect = async (themeName: string) => {
    selectedLightTheme.value = themeName
    if (currentMode.value === 'system') {
      await themeStore.setFollowSystem(true, themeName, selectedDarkTheme.value)
    }
  }

  const handleDarkThemeSelect = async (themeName: string) => {
    selectedDarkTheme.value = themeName
    if (currentMode.value === 'system') {
      await themeStore.setFollowSystem(true, selectedLightTheme.value, themeName)
    }
  }
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('settings.theme.title') }}</h1>

    <!-- Theme Mode -->
    <div class="settings-group">
      <h2 class="group-title">{{ t('theme_settings.appearance') }}</h2>

      <div class="settings-card">
        <div class="settings-row">
          <div class="row-info">
            <span class="row-label">{{ t('theme_settings.theme_mode') }}</span>
            <span class="row-description">{{ t('theme_settings.theme_mode_description') }}</span>
          </div>
          <div class="row-control">
            <div class="segment-control">
              <button
                class="segment-btn"
                :class="{ active: currentMode === 'light' }"
                @click="handleModeChange('light')"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="5" />
                  <line x1="12" y1="1" x2="12" y2="3" />
                  <line x1="12" y1="21" x2="12" y2="23" />
                  <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                  <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                  <line x1="1" y1="12" x2="3" y2="12" />
                  <line x1="21" y1="12" x2="23" y2="12" />
                </svg>
                {{ t('theme_settings.light') }}
              </button>
              <button class="segment-btn" :class="{ active: currentMode === 'dark' }" @click="handleModeChange('dark')">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                </svg>
                {{ t('theme_settings.dark') }}
              </button>
              <button
                class="segment-btn"
                :class="{ active: currentMode === 'system' }"
                @click="handleModeChange('system')"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                  <line x1="8" y1="21" x2="16" y2="21" />
                  <line x1="12" y1="17" x2="12" y2="21" />
                </svg>
                {{ t('theme_settings.system') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Manual Theme Selection -->
    <div v-if="currentMode !== 'system'" class="settings-group">
      <h2 class="group-title">{{ t('theme_settings.select_theme') }}</h2>

      <div class="theme-grid">
        <div
          v-for="option in manualThemeOptions"
          :key="option.value"
          class="theme-card"
          :class="{ selected: option.isCurrent }"
          @click="handleThemeSelect(option.value)"
        >
          <div class="theme-preview" :style="{ background: getThemeColors(option).bg }">
            <div class="preview-header">
              <div class="preview-dots">
                <span :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
              </div>
            </div>
            <div class="preview-content">
              <div class="preview-sidebar" :style="{ background: getThemeColors(option).sidebar }"></div>
              <div class="preview-main" :style="{ background: getThemeColors(option).card }">
                <div
                  class="preview-line short"
                  :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"
                ></div>
                <div class="preview-line" :style="{ background: getThemeColors(option).line }"></div>
                <div class="preview-line medium" :style="{ background: getThemeColors(option).line }"></div>
              </div>
            </div>
          </div>
          <div class="theme-info">
            <span class="theme-name">{{ option.label }}</span>
            <svg
              v-if="option.isCurrent"
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

    <!-- System Theme - Light -->
    <div v-if="currentMode === 'system'" class="settings-group">
      <h2 class="group-title">{{ t('theme_settings.light_theme') }}</h2>

      <div class="theme-grid">
        <div
          v-for="option in systemLightThemeOptions"
          :key="option.value"
          class="theme-card"
          :class="{ selected: selectedLightTheme === option.value }"
          @click="handleLightThemeSelect(option.value)"
        >
          <div class="theme-preview" :style="{ background: getThemeColors(option).bg }">
            <div class="preview-header">
              <div class="preview-dots">
                <span :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
              </div>
            </div>
            <div class="preview-content">
              <div class="preview-sidebar" :style="{ background: getThemeColors(option).sidebar }"></div>
              <div class="preview-main" :style="{ background: getThemeColors(option).card }">
                <div
                  class="preview-line short"
                  :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"
                ></div>
                <div class="preview-line" :style="{ background: getThemeColors(option).line }"></div>
                <div class="preview-line medium" :style="{ background: getThemeColors(option).line }"></div>
              </div>
            </div>
          </div>
          <div class="theme-info">
            <span class="theme-name">{{ option.label }}</span>
            <svg
              v-if="selectedLightTheme === option.value"
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

    <!-- System Theme - Dark -->
    <div v-if="currentMode === 'system'" class="settings-group">
      <h2 class="group-title">{{ t('theme_settings.dark_theme') }}</h2>

      <div class="theme-grid">
        <div
          v-for="option in systemDarkThemeOptions"
          :key="option.value"
          class="theme-card"
          :class="{ selected: selectedDarkTheme === option.value }"
          @click="handleDarkThemeSelect(option.value)"
        >
          <div class="theme-preview" :style="{ background: getThemeColors(option).bg }">
            <div class="preview-header">
              <div class="preview-dots">
                <span :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
                <span :style="{ background: getThemeColors(option).text, opacity: 0.3 }"></span>
              </div>
            </div>
            <div class="preview-content">
              <div class="preview-sidebar" :style="{ background: getThemeColors(option).sidebar }"></div>
              <div class="preview-main" :style="{ background: getThemeColors(option).card }">
                <div
                  class="preview-line short"
                  :style="{ background: getThemeColors(option).primary, opacity: 0.8 }"
                ></div>
                <div class="preview-line" :style="{ background: getThemeColors(option).line }"></div>
                <div class="preview-line medium" :style="{ background: getThemeColors(option).line }"></div>
              </div>
            </div>
          </div>
          <div class="theme-info">
            <span class="theme-name">{{ option.label }}</span>
            <svg
              v-if="selectedDarkTheme === option.value"
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

  /* Segment Control */
  .segment-control {
    display: flex;
    background: var(--bg-200);
    border-radius: var(--border-radius-lg);
    padding: 2px;
    gap: 2px;
  }

  .segment-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .segment-btn:hover {
    color: var(--text-200);
  }

  .segment-btn.active {
    background: var(--bg-100);
    color: var(--text-100);
    box-shadow: var(--shadow-sm);
  }

  /* Theme Grid */
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  .theme-card {
    background: var(--bg-100);
    border: 2px solid var(--border-100);
    border-radius: var(--border-radius-xl);
    overflow: hidden;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .theme-card:hover {
    border-color: var(--border-300);
  }

  .theme-card.selected {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 20%, transparent);
  }

  .theme-preview {
    aspect-ratio: 16 / 10;
    padding: 6px;
    display: flex;
    flex-direction: column;
    border-radius: var(--border-radius-xl) var(--border-radius-xl) 0 0;
  }

  .preview-header {
    height: 10px;
    display: flex;
    align-items: center;
    padding: 0 4px;
    margin-bottom: 4px;
  }

  .preview-dots {
    display: flex;
    gap: 3px;
  }

  .preview-dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
  }

  .preview-content {
    flex: 1;
    display: flex;
    gap: 4px;
    border-radius: var(--border-radius-sm);
    overflow: hidden;
  }

  .preview-sidebar {
    width: 25%;
    border-radius: var(--border-radius-xs);
  }

  .preview-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px;
    border-radius: var(--border-radius-xs);
  }

  .preview-line {
    height: 4px;
    border-radius: var(--border-radius-xs);
  }

  .preview-line.short {
    width: 40%;
  }

  .preview-line.medium {
    width: 70%;
  }

  .theme-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-top: 1px solid var(--border-100);
  }

  .theme-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-200);
  }

  .theme-card.selected .theme-name {
    color: var(--color-primary);
  }

  .check-icon {
    width: 16px;
    height: 16px;
    color: var(--color-primary);
  }
</style>
