<script setup lang="ts">
  import { useShortcutStore } from '@/stores/shortcuts'
  import type { ShortcutBinding } from '@/types'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  const store = useShortcutStore()
  const config = computed(() => store.config)
  const loading = computed(() => store.loading)

  const initialize = async () => {
    if (!store.initialized || !store.config) await store.refreshConfig()
  }
  const addShortcut = (shortcut: ShortcutBinding) => store.addShortcut(shortcut)
  const removeShortcut = (index: number) => store.removeShortcut(index)
  const resetToDefaults = () => store.resetToDefaults()
  const { t } = useI18n()

  const editingActionKey = ref<string | null>(null)
  const capturedShortcut = ref<{ key: string; modifiers: string[] } | null>(null)

  onMounted(async () => {
    await initialize()
  })

  const handleReset = async () => {
    await resetToDefaults()
    await (window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts?.()
  }

  const confirmReset = async () => {
    const confirmed = await confirm(t('shortcuts.reset_confirm_message'), {
      title: t('shortcuts.reset_confirm_title'),
      kind: 'warning',
    })
    if (confirmed) {
      handleReset()
    }
  }

  const allActionKeys = [
    'copy_to_clipboard',
    'paste_from_clipboard',
    'command_palette',
    'terminal_search',
    'open_settings',
    'new_terminal',
    'clear_terminal',
    'accept_completion',
    'toggle_terminal_panel',
    'toggle_window_pin',
  ]

  const findShortcut = (actionKey: string) => {
    if (!config.value) return null
    for (const shortcut of config.value) {
      if (shortcut.action === actionKey) return shortcut
    }
    return null
  }

  const allActions = computed(() => {
    return allActionKeys.map(actionKey => ({
      key: actionKey,
      displayName: t(`shortcuts.actions.${actionKey}`) || actionKey,
      shortcut: findShortcut(actionKey),
    }))
  })

  const isEditing = (actionKey: string) => editingActionKey.value === actionKey

  const startEdit = (actionKey: string) => {
    editingActionKey.value = actionKey
    capturedShortcut.value = null
  }

  const stopEdit = (actionKey: string) => {
    if (editingActionKey.value === actionKey) {
      editingActionKey.value = null
      if (capturedShortcut.value) {
        saveShortcut(actionKey, capturedShortcut.value)
      }
      capturedShortcut.value = null
    }
  }

  const handleKeyDown = (event: KeyboardEvent, actionKey: string) => {
    if (!isEditing(actionKey)) return

    event.preventDefault()
    event.stopPropagation()

    const modifiers: string[] = []
    if (event.ctrlKey) modifiers.push('ctrl')
    if (event.metaKey) modifiers.push('cmd')
    if (event.altKey) modifiers.push('alt')
    if (event.shiftKey) modifiers.push('shift')

    let key = event.key
    if (key === ' ') key = 'Space'
    if (key === 'Control' || key === 'Meta' || key === 'Alt' || key === 'Shift') return

    capturedShortcut.value = { key, modifiers }
    setTimeout(() => stopEdit(actionKey), 100)
  }

  const saveShortcut = async (actionKey: string, shortcut: { key: string; modifiers: string[] }) => {
    const shortcutBinding: ShortcutBinding = {
      key: shortcut.key,
      modifiers: shortcut.modifiers,
      action: actionKey,
    }

    await removeExistingShortcut(actionKey)
    await addShortcut(shortcutBinding)
    await (window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts?.()
  }

  const removeExistingShortcut = async (actionKey: string) => {
    if (!config.value) return
    for (let i = 0; i < config.value.length; i++) {
      if (config.value[i].action === actionKey) {
        await removeShortcut(i)
        return
      }
    }
  }

  const formatModifier = (modifier: string) => {
    const modifierMap: Record<string, string> = {
      cmd: '⌘',
      ctrl: '⌃',
      alt: '⌥',
      shift: '⇧',
    }
    return modifierMap[modifier] || modifier
  }

  const formatKey = (key: string) => {
    const keyMap: Record<string, string> = {
      ArrowUp: '↑',
      ArrowDown: '↓',
      ArrowLeft: '←',
      ArrowRight: '→',
      Enter: '↵',
      Escape: 'Esc',
      Backspace: '⌫',
      Delete: '⌦',
      Tab: '⇥',
      Space: '␣',
    }
    return keyMap[key] || key.toUpperCase()
  }

  const init = async () => {
    await initialize()
  }

  defineExpose({ init })
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('shortcuts.title') }}</h1>

    <!-- Loading State -->
    <div v-if="loading" class="settings-loading">
      <div class="loading-spinner"></div>
      <span>{{ t('shortcuts.loading') }}</span>
    </div>

    <template v-else>
      <!-- Shortcuts List -->
      <div class="settings-group">
        <h2 class="group-title">{{ t('shortcuts.keyboard_shortcuts') }}</h2>

        <div class="settings-card">
          <div
            v-for="action in allActions"
            :key="action.key"
            class="settings-row clickable"
            :class="{ editing: isEditing(action.key) }"
            tabindex="0"
            @click="startEdit(action.key)"
            @keydown="handleKeyDown($event, action.key)"
            @blur="stopEdit(action.key)"
          >
            <div class="row-info">
              <span class="row-label">{{ action.displayName }}</span>
            </div>
            <div class="row-control">
              <template v-if="!isEditing(action.key)">
                <div v-if="action.shortcut" class="shortcut-keys">
                  <kbd v-for="modifier in action.shortcut.modifiers" :key="modifier" class="key">
                    {{ formatModifier(modifier) }}
                  </kbd>
                  <kbd class="key">{{ formatKey(action.shortcut.key) }}</kbd>
                </div>
                <span v-else class="shortcut-empty">{{ t('shortcuts.click_to_set') }}</span>
              </template>
              <span v-else class="shortcut-recording">
                <span class="recording-dot"></span>
                {{ t('shortcuts.press_keys') }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Reset Section -->
      <div class="settings-group">
        <h2 class="group-title">{{ t('shortcuts.reset_title') }}</h2>

        <div class="settings-card">
          <div class="settings-row">
            <div class="row-info">
              <span class="row-label">{{ t('shortcuts.reset_shortcuts') }}</span>
              <span class="row-description">{{ t('shortcuts.reset_description') }}</span>
            </div>
            <div class="row-control">
              <button class="reset-btn" :disabled="loading" @click="confirmReset">
                {{ t('shortcuts.reset_to_default') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </template>
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
    padding: 14px 20px;
    min-height: 52px;
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

  .settings-row.clickable:focus {
    outline: none;
    background: var(--bg-300);
  }

  .settings-row.editing {
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

  /* Shortcut Keys */
  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .key {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    background: var(--bg-300);
    border-radius: var(--border-radius-sm);
    box-shadow: 0 1px 0 var(--border-300);
    font-family: var(--font-family);
  }

  .shortcut-empty {
    font-size: 12px;
    color: var(--text-500);
    font-style: italic;
  }

  .shortcut-recording {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-primary);
  }

  .recording-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: recording-pulse 1s ease-in-out infinite;
  }

  @keyframes recording-pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(0.8);
    }
  }

  /* Reset Button */
  .reset-btn {
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .reset-btn:hover {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  .reset-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Loading */
  .settings-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 48px;
    color: var(--text-400);
  }

  .loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-200);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
