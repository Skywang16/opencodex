<script setup lang="ts">
  import type { RunActionRecord } from '@/stores/runActions'
  import { XButton, XFormGroup, XInput, XModal, XTextarea } from '@/ui'
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

      <XFormGroup :label="t('run_actions.name')" required>
        <XInput v-model="name" :placeholder="t('run_actions.name_placeholder')" @enter="handleSave" />
      </XFormGroup>

      <XFormGroup :label="t('run_actions.command')" required>
        <XTextarea v-model="command" :placeholder="t('run_actions.command_placeholder')" :rows="4" />
      </XFormGroup>
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

  .dialog-footer {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
</style>
