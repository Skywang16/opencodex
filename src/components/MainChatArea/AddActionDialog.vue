<script setup lang="ts">
  import type { RunActionRecord } from '@/stores/runActions'
  import { XButton, XModal } from '@/ui'
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    visible: boolean
    editAction?: RunActionRecord | null
  }

  interface Emits {
    (e: 'update:visible', value: boolean): void
    (e: 'save', action: { name: string; command: string }): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const { t } = useI18n()

  const name = ref('')
  const command = ref('')

  const isEditing = computed(() => !!props.editAction)

  const canSave = computed(() => {
    return name.value.trim().length > 0 && command.value.trim().length > 0
  })

  watch(
    () => props.visible,
    visible => {
      if (visible) {
        if (props.editAction) {
          name.value = props.editAction.name
          command.value = props.editAction.command
        } else {
          name.value = ''
          command.value = ''
        }
      }
    }
  )

  const handleClose = () => {
    emit('update:visible', false)
  }

  const handleSave = () => {
    if (!canSave.value) return

    emit('save', {
      name: name.value.trim(),
      command: command.value.trim(),
    })
    emit('update:visible', false)
  }
</script>

<template>
  <XModal
    :visible="visible"
    :title="isEditing ? t('run_actions.edit_action') : t('run_actions.add_action')"
    size="small"
    :show-footer="true"
    :show-header="true"
    @update:visible="emit('update:visible', $event)"
    @close="handleClose"
  >
    <div class="dialog-content">
      <p class="dialog-description">{{ t('run_actions.add_action_description') }}</p>

      <div class="form-group">
        <label class="form-label">{{ t('run_actions.name') }}</label>
        <div class="input-wrapper">
          <div class="input-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="3" />
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.09a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.09a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
              />
            </svg>
          </div>
          <input
            v-model="name"
            type="text"
            class="form-input"
            :placeholder="t('run_actions.name_placeholder')"
            @keydown.enter="handleSave"
          />
        </div>
      </div>

      <div class="form-group">
        <label class="form-label">{{ t('run_actions.command') }}</label>
        <textarea
          v-model="command"
          class="form-textarea"
          :placeholder="t('run_actions.command_placeholder')"
          rows="4"
        />
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <XButton variant="primary" size="medium" :disabled="!canSave" @click="handleSave">
          {{ t('common.save') }}
        </XButton>
      </div>
    </template>
  </XModal>
</template>

<style scoped>
  .dialog-content {
    padding-top: var(--spacing-sm);
  }

  .dialog-description {
    font-size: 13px;
    color: var(--text-400);
    margin: 0 0 20px;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-300);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .input-wrapper {
    display: flex;
    align-items: center;
    background: var(--bg-200);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-xl);
    transition: all 0.15s ease;
  }

  .input-wrapper:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
  }

  .input-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    color: var(--text-400);
  }

  .input-icon svg {
    width: 18px;
    height: 18px;
  }

  .form-input {
    flex: 1;
    height: 40px;
    padding: 0 12px 0 0;
    background: transparent;
    border: none;
    font-size: 14px;
    color: var(--text-100);
    outline: none;
  }

  .form-input::placeholder {
    color: var(--text-500);
  }

  .form-textarea {
    width: 100%;
    padding: 12px;
    background: var(--bg-200);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-xl);
    font-size: 13px;
    font-family: var(--font-family-mono);
    color: var(--text-200);
    resize: vertical;
    outline: none;
    transition: all 0.15s ease;
  }

  .form-textarea:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
  }

  .form-textarea::placeholder {
    color: var(--text-500);
  }

  .dialog-footer {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
</style>
