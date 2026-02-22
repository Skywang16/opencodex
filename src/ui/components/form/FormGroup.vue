<template>
  <div class="x-form-group" :class="formGroupClasses">
    <label v-if="label || $slots.label" class="x-form-group__label" :for="labelFor">
      <slot name="label">{{ label }}</slot>
      <span v-if="required" class="x-form-group__required" aria-hidden="true">*</span>
    </label>

    <div class="x-form-group__content">
      <slot></slot>
    </div>

    <div v-if="showMessage" class="x-form-group__message">
      <transition name="x-form-group__message-fade">
        <p v-if="error" class="x-form-group__error" role="alert">{{ error }}</p>
        <p v-else-if="hint && !error" class="x-form-group__hint">{{ hint }}</p>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, provide } from 'vue'
  import type { FormGroupProps, FormStatus } from './types'

  const props = withDefaults(defineProps<FormGroupProps>(), {
    label: '',
    required: false,
    error: '',
    hint: '',
    status: 'default',
    size: 'medium',
    disabled: false,
  })

  defineSlots<{
    label?: () => unknown
    default?: () => unknown
  }>()

  // Generate a unique ID for label association
  const labelFor = computed(() => {
    return props.label ? `x-form-group-${Math.random().toString(36).slice(2, 9)}` : undefined
  })

  const showMessage = computed(() => props.error || props.hint)

  const formGroupClasses = computed(() => ({
    [`x-form-group--${props.size}`]: props.size !== 'medium',
    [`x-form-group--${props.status}`]: props.status !== 'default',
    'x-form-group--disabled': props.disabled,
    'x-form-group--has-error': !!props.error,
  }))

  // Provide form group context to child form controls
  provide('formGroup', {
    size: computed(() => props.size),
    disabled: computed(() => props.disabled),
    status: computed(() => (props.error ? 'error' : props.status) as FormStatus),
  })
</script>

<style scoped>
  .x-form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 0;
  }

  /* Size variants */
  .x-form-group--small .x-form-group__label {
    font-size: 12px;
  }

  .x-form-group--large .x-form-group__label {
    font-size: 14px;
  }

  /* Label */
  .x-form-group__label {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    line-height: 1.4;
  }

  .x-form-group__required {
    color: var(--color-error);
    font-weight: 500;
  }

  /* Content wrapper */
  .x-form-group__content {
    position: relative;
  }

  /* Message area */
  .x-form-group__message {
    min-height: 18px;
    line-height: 18px;
  }

  .x-form-group__error,
  .x-form-group__hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }

  .x-form-group__error {
    color: var(--color-error);
  }

  .x-form-group__hint {
    color: var(--text-400);
  }

  /* Status variants */
  .x-form-group--error .x-form-group__label {
    color: var(--color-error);
  }

  .x-form-group--success .x-form-group__label {
    color: var(--color-success);
  }

  .x-form-group--warning .x-form-group__label {
    color: var(--color-warning);
  }

  /* Disabled state */
  .x-form-group--disabled .x-form-group__label {
    color: var(--text-400);
    cursor: not-allowed;
  }

  /* Message transition */
  .x-form-group__message-fade-enter-active,
  .x-form-group__message-fade-leave-active {
    transition: opacity 0.15s ease;
  }

  .x-form-group__message-fade-enter-from,
  .x-form-group__message-fade-leave-to {
    opacity: 0;
  }
</style>
