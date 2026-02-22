<template>
  <div class="x-textarea" :class="textareaClasses">
    <textarea
      ref="textareaRef"
      v-model="inputValue"
      class="x-textarea__inner"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :rows="computedRows"
      :maxlength="maxlength"
      :autofocus="autofocus"
      :name="name"
      :id="id"
      :style="textareaStyle"
      :aria-disabled="disabled"
      :aria-invalid="status === 'error'"
      @input="handleInput"
      @change="handleChange"
      @focus="handleFocus"
      @blur="handleBlur"
      @keydown="handleKeydown"
    ></textarea>

    <!-- Word limit -->
    <span v-if="showWordLimit" class="x-textarea__count">
      <span class="x-textarea__count-current">{{ currentLength }}</span>
      <span class="x-textarea__count-separator">/</span>
      <span class="x-textarea__count-max">{{ maxlength }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, nextTick, onMounted, ref, watch, type ComputedRef } from 'vue'
import type { TextareaEmits, TextareaProps, FormStatus, Size } from './types'

const props = withDefaults(defineProps<TextareaProps>(), {
  modelValue: '',
  placeholder: '',
  disabled: false,
  readonly: false,
  rows: 3,
  autosize: false,
  resize: 'vertical',
  status: 'default',
  autofocus: false,
})

const emit = defineEmits<TextareaEmits>()

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

const textareaRef = ref<HTMLTextAreaElement>()

// Computed status from form group
const computedDisabled = computed(() => props.disabled || formGroup.disabled.value)
const computedStatus = computed(() => {
  if (props.status !== 'default') return props.status
  return formGroup.status.value
})

// Input value
const inputValue = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val ?? ''),
})

// Current length for word limit
const currentLength = computed(() => String(props.modelValue ?? '').length)

// Show word limit
const showWordLimit = computed(() => props.maxlength && props.showWordLimit)

// Computed rows for autosize
const computedRows = computed(() => {
  if (props.autosize) {
    return 1
  }
  return props.rows
})

// Textarea inline style
const textareaStyle = computed(() => ({
  resize: props.resize,
}))

// Textarea classes
const textareaClasses = computed(() => ({
  [`x-textarea--${computedStatus.value}`]: computedStatus.value !== 'default',
  'x-textarea--disabled': computedDisabled.value,
  'x-textarea--readonly': props.readonly,
  'x-textarea--autosize': props.autosize,
  'x-textarea--has-count': showWordLimit.value,
}))

// Autosize logic
const calculateHeight = () => {
  if (!textareaRef.value || !props.autosize) return

  const textarea = textareaRef.value
  textarea.style.height = 'auto'

  let minHeight: number
  let maxHeight: number | undefined

  if (typeof props.autosize === 'object') {
    const lineHeight = parseInt(getComputedStyle(textarea).lineHeight) || 20
    const paddingTop = parseInt(getComputedStyle(textarea).paddingTop) || 0
    const paddingBottom = parseInt(getComputedStyle(textarea).paddingBottom) || 0
    const baseHeight = paddingTop + paddingBottom

    minHeight = baseHeight + lineHeight * (props.autosize.minRows || 1)
    maxHeight = props.autosize.maxRows ? baseHeight + lineHeight * props.autosize.maxRows : undefined
  } else {
    minHeight = textarea.scrollHeight
  }

  textarea.style.height = `${Math.max(textarea.scrollHeight, minHeight)}px`

  if (maxHeight && textarea.scrollHeight > maxHeight) {
    textarea.style.height = `${maxHeight}px`
    textarea.style.overflowY = 'auto'
  } else {
    textarea.style.overflowY = 'hidden'
  }
}

// Watch value changes for autosize
watch(
  () => props.modelValue,
  () => {
    nextTick(calculateHeight)
  }
)

onMounted(() => {
  calculateHeight()
})

// Event handlers
const handleInput = (event: Event) => {
  const value = (event.target as HTMLTextAreaElement).value
  emit('input', value)
}

const handleChange = (event: Event) => {
  const value = (event.target as HTMLTextAreaElement).value
  emit('change', value)
}

const handleFocus = (event: FocusEvent) => {
  emit('focus', event)
}

const handleBlur = (event: FocusEvent) => {
  emit('blur', event)
}

const handleKeydown = (event: KeyboardEvent) => {
  emit('keydown', event)
}

// Expose methods
defineExpose({
  focus: () => textareaRef.value?.focus(),
  blur: () => textareaRef.value?.blur(),
  select: () => textareaRef.value?.select(),
  textarea: textareaRef,
  resize: calculateHeight,
})
</script>

<style scoped>
.x-textarea {
  display: inline-flex;
  flex-direction: column;
  width: 100%;
  position: relative;
  background: var(--bg-200);
  border: 1px solid var(--border-200);
  border-radius: var(--border-radius-lg);
  transition: all 0.15s ease;
}

.x-textarea:hover:not(.x-textarea--disabled) {
  border-color: var(--border-300);
}

.x-textarea:focus-within:not(.x-textarea--disabled) {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
}

/* Textarea element */
.x-textarea__inner {
  width: 100%;
  padding: 10px 12px;
  background: transparent;
  border: none;
  outline: none;
  font-size: 14px;
  font-family: var(--font-family-mono);
  color: var(--text-100);
  line-height: 1.5;
  resize: vertical;
  min-height: 80px;
}

.x-textarea__inner::placeholder {
  color: var(--text-500);
}

/* Word count */
.x-textarea__count {
  display: flex;
  justify-content: flex-end;
  padding: 4px 12px 8px;
  font-size: 12px;
  line-height: 1;
  color: var(--text-400);
}

.x-textarea__count-current {
  color: var(--text-300);
}

.x-textarea__count-separator {
  margin: 0 2px;
}

/* Status variants */
.x-textarea--error {
  border-color: var(--color-error);
}

.x-textarea--error:focus-within {
  border-color: var(--color-error);
  box-shadow: 0 0 0 3px rgba(var(--color-error-rgb), 0.1);
}

.x-textarea--success {
  border-color: var(--color-success);
}

.x-textarea--success:focus-within {
  border-color: var(--color-success);
  box-shadow: 0 0 0 3px rgba(var(--color-success-rgb), 0.1);
}

.x-textarea--warning {
  border-color: var(--color-warning);
}

.x-textarea--warning:focus-within {
  border-color: var(--color-warning);
  box-shadow: 0 0 0 3px rgba(var(--color-warning-rgb), 0.1);
}

/* Disabled state */
.x-textarea--disabled {
  cursor: not-allowed;
  background: var(--bg-300);
  opacity: 0.6;
}

.x-textarea--disabled .x-textarea__inner {
  cursor: not-allowed;
  color: var(--text-400);
}

/* Readonly state */
.x-textarea--readonly .x-textarea__inner {
  cursor: default;
}

/* Autosize */
.x-textarea--autosize .x-textarea__inner {
  overflow: hidden;
  resize: none;
}

/* Has count */
.x-textarea--has-count .x-textarea__inner {
  padding-bottom: 4px;
}
</style>
