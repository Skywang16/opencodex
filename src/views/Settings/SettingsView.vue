<script setup lang="ts">
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import { GeneralSettings } from '@/components/settings/components/General'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import { McpServersSettings } from '@/components/settings/components/Mcp'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import { computed } from 'vue'

  const props = defineProps<{
    activeSection: string
  }>()

  const components = {
    general: GeneralSettings,
    ai: AISettings,
    mcp: McpServersSettings,
    theme: ThemeSettings,
    shortcuts: ShortcutSettings,
    language: LanguageSettings,
  }

  const currentComponent = computed(() => {
    return components[props.activeSection as keyof typeof components] || GeneralSettings
  })
</script>

<template>
  <div class="settings-scroll">
    <div class="settings-content">
      <component :is="currentComponent" />
    </div>
  </div>
</template>

<style scoped>
  .settings-scroll {
    flex: 1;
    overflow-y: auto;
    padding-top: 40px;
  }

  .settings-scroll::-webkit-scrollbar {
    width: 8px;
  }

  .settings-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .settings-scroll::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-sm);
  }

  .settings-scroll::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  .settings-content {
    max-width: 720px;
    margin: 0 auto;
    padding: 16px 48px 32px;
  }
</style>
