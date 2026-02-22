<template>
  <div class="x-input" :class="inputClasses">
    <!-- Prefix slot -->
    <span v-if="$slots.prefix" class="x-input__prefix">
      <slot name="prefix"></slot>
    </span>

    <!-- Input element -->
    <input
      ref="inputRef"
      v-model="inputValue"
      class="x-input__inner"
      :type="type"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :maxlength="maxlength"
      :autofocus="autofocus"
      :autocomplete="autocomplete"
      :name="name"
      :id="id"
      :step="step"
      :min="min"
      :max="max"
      :aria-disabled="disabled"
      :aria-invalid="status === 'error'"
      @input="handleInput"
      @change="handleChange"
      @focus="handleFocus"
      @blur="handleBlur"
      @keydown="handleKeydown"
    />

    <!-- Suffix slot / Clear button / Word limit -->
    <span v-if="showSuffix" class="x-input__suffix">
      <!-- Clear button -->
      <button
        v-if="clearable && hasValue && !disabled && !readonly"
        type="button"
        class="x-input__clear"
        tabindex="-1"
        :aria-label="'Clear'"
        @click.stop="handleClear"
        @mousedown.prevent
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
      </button>

      <!-- Word limit -->
      <span v-if="showWordLimit" class="x-input__count">
        <span class="x-input__count-current">{{ currentLength }}</span>
        <span class="x-input__count-separator">/</span>
        <span class="x-input__count-max">{{ maxlength }}</span>
      </span>

      <!-- Custom suffix slot -->
      <slot name="suffix"></slot>
    </span>
  </div>
</template>

<script setup lang="ts">
  import { computed, inject, ref, useSlots, type ComputedRef } from 'vue'
  import type { InputEmits, InputProps, FormStatus, Size } from './types'

  const props = withDefaults(defineProps<InputProps>(), {
    modelValue: '',
    type: 'text',
    placeholder: '',
    disabled: false,
    readonly: false,
    size: 'medium',
    status: 'default',
    clearable: false,
    showWordLimit: false,
    autofocus: false,
    autocomplete: 'off',
  })

  const emit = defineEmits<InputEmits>()

  const slots = useSlots()

  defineSlots<{
    prefix?: () => unknown
    suffix?: () => unknown
  }>()

  // Inject form group context if available
  const formGroup = inject<{
    size: ComputedRef<Size>
    disabled: ComputedRef<boolean>
    status: ComputedRef<FormStatus>
  }>('formGroup', {
    size: ref('medium') as ComputedRef<Size>,
    disabled: ref(false) as ComputedRef<boolean>,
    status: ref('default') as ComputedRef<FormStatus>,
  })

  const inputRef = ref<HTMLInputElement>()

  // Computed size and status from form group
  const computedSize = computed(() => props.size ?? formGroup.size.value)
  const computedDisabled = computed(() => props.disabled || formGroup.disabled.value)
  const computedStatus = computed(() => {
    if (props.status !== 'default') return props.status
    return formGroup.status.value
  })

  // Input value with type handling
  const inputValue = computed({
    get: () => props.modelValue,
    set: val => emit('update:modelValue', val ?? ''),
  })

  // Current length for word limit
  const currentLength = computed(() => String(props.modelValue ?? '').length)
  const hasValue = computed(() => currentLength.value > 0)

  // Show suffix area
  const showSuffix = computed(() => {
    return (
      (props.clearable && hasValue.value && !computedDisabled.value && !props.readonly) ||
      props.showWordLimit ||
      Boolean(slots.suffix)
    )
  })

  // Input classes
  const inputClasses = computed(() => ({
    [`x-input--${computedSize.value}`]: computedSize.value !== 'medium',
    [`x-input--${computedStatus.value}`]: computedStatus.value !== 'default',
    'x-input--disabled': computedDisabled.value,
    'x-input--readonly': props.readonly,
    'x-input--clearable': props.clearable,
    'x-input--has-prefix': Boolean(slots.prefix),
    'x-input--has-suffix': showSuffix.value,
  }))

  // Event handlers
  const handleInput = (event: Event) => {
    const value = (event.target as HTMLInputElement).value
    emit('input', props.type === 'number' ? Number(value) : value)
  }

  const handleChange = (event: Event) => {
    const value = (event.target as HTMLInputElement).value
    emit('change', props.type === 'number' ? Number(value) : value)
  }

  const handleFocus = (event: FocusEvent) => {
    emit('focus', event)
  }

  const handleBlur = (event: FocusEvent) => {
    emit('blur', event)
  }

  const handleKeydown = (event: KeyboardEvent) => {
    emit('keydown', event)
    if (event.key === 'Enter') {
      emit('enter', event)
    }
  }

  const handleClear = () => {
    emit('update:modelValue', props.type === 'number' ? 0 : '')
    emit('clear')
    inputRef.value?.focus()
  }

  // Expose methods
  defineExpose({
    focus: () => inputRef.value?.focus(),
    blur: () => inputRef.value?.blur(),
    select: () => inputRef.value?.select(),
    input: inputRef,
  })
</script>

<style scoped>
  .x-input {
    display: inline-flex;
    align-items: center;
    width: 100%;
    position: relative;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    transition: all 0.15s ease;
  }

  .x-input:hover:not(.x-input--disabled) {
    border-color: var(--border-300);
  }

  .x-input:focus-within:not(.x-input--disabled) {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
  }

  /* Input element */
  .x-input__inner {
    flex: 1;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: none;
    outline: none;
    font-size: 14px;
    font-family: inherit;
    color: var(--text-100);
    line-height: 1.4;
  }

  .x-input__inner::placeholder {
    color: var(--text-500);
  }

  .x-input__inner::-webkit-outer-spin-button,
  .x-input__inner::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .x-input__inner[type='number'] {
    -moz-appearance: textfield;
  }

  /* Prefix & Suffix */
  .x-input__prefix,
  .x-input__suffix {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--text-400);
  }

  .x-input__prefix {
    padding-left: 12px;
  }

  .x-input__suffix {
    padding-right: 12px;
    gap: 8px;
  }

  .x-input--has-prefix .x-input__inner {
    padding-left: 4px;
  }

  .x-input--has-suffix .x-input__inner {
    padding-right: 4px;
  }

  /* Clear button */
  .x-input__clear {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    background: var(--bg-300);
    border: none;
    border-radius: 50%;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .x-input__clear:hover {
    background: var(--bg-400);
    color: var(--text-200);
  }

  .x-input__clear svg {
    width: 10px;
    height: 10px;
  }

  /* Word count */
  .x-input__count {
    display: inline-flex;
    align-items: center;
    font-size: 12px;
    line-height: 1;
    color: var(--text-400);
    white-space: nowrap;
  }

  .x-input__count-current {
    color: var(--text-300);
  }

  .x-input__count-separator {
    margin: 0 2px;
  }

  /* Size variants */
  .x-input--small .x-input__inner {
    padding: 6px 10px;
    font-size: 13px;
  }

  .x-input--small .x-input__prefix {
    padding-left: 10px;
  }

  .x-input--small .x-input__suffix {
    padding-right: 10px;
  }

  .x-input--large .x-input__inner {
    padding: 12px 14px;
    font-size: 15px;
  }

  .x-input--large .x-input__prefix {
    padding-left: 14px;
  }

  .x-input--large .x-input__suffix {
    padding-right: 14px;
  }

  /* Status variants */
  .x-input--error {
    border-color: var(--color-error);
  }

  .x-input--error:focus-within {
    border-color: var(--color-error);
    box-shadow: 0 0 0 3px rgba(var(--color-error-rgb), 0.1);
  }

  .x-input--success {
    border-color: var(--color-success);
  }

  .x-input--success:focus-within {
    border-color: var(--color-success);
    box-shadow: 0 0 0 3px rgba(var(--color-success-rgb), 0.1);
  }

  .x-input--warning {
    border-color: var(--color-warning);
  }

  .x-input--warning:focus-within {
    border-color: var(--color-warning);
    box-shadow: 0 0 0 3px rgba(var(--color-warning-rgb), 0.1);
  }

  /* Disabled state */
  .x-input--disabled {
    cursor: not-allowed;
    background: var(--bg-300);
    opacity: 0.6;
  }

  .x-input--disabled .x-input__inner {
    cursor: not-allowed;
    color: var(--text-400);
  }

  /* Readonly state */
  .x-input--readonly .x-input__inner {
    cursor: default;
  }
</style>
