/**
 * Form component type definitions
 */

/** Size type */
export type Size = 'small' | 'medium' | 'large'

/** Form item validation status */
export type FormStatus = 'default' | 'error' | 'success' | 'warning'

/** Form item props */
export interface FormGroupProps {
  label?: string
  required?: boolean
  error?: string
  hint?: string
  status?: FormStatus
  size?: Size
  disabled?: boolean
}

/** Input component props */
export interface InputProps {
  modelValue?: string | number
  type?: 'text' | 'password' | 'email' | 'number' | 'tel' | 'url' | 'search'
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  size?: Size
  status?: FormStatus
  clearable?: boolean
  maxlength?: number
  showWordLimit?: boolean
  autofocus?: boolean
  autocomplete?: string
  name?: string
  id?: string
  step?: string | number
  min?: number
  max?: number
}

/** Input component emits */
export interface InputEmits {
  (e: 'update:modelValue', value: string | number): void
  (e: 'input', value: string | number): void
  (e: 'change', value: string | number): void
  (e: 'focus', event: FocusEvent): void
  (e: 'blur', event: FocusEvent): void
  (e: 'clear'): void
  (e: 'enter', event: KeyboardEvent): void
  (e: 'keydown', event: KeyboardEvent): void
}

/** Textarea component props */
export interface TextareaProps {
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  rows?: number
  autosize?: boolean | { minRows?: number; maxRows?: number }
  maxlength?: number
  showWordLimit?: boolean
  resize?: 'none' | 'both' | 'horizontal' | 'vertical'
  status?: FormStatus
  autofocus?: boolean
  name?: string
  id?: string
}

/** Textarea component emits */
export interface TextareaEmits {
  (e: 'update:modelValue', value: string): void
  (e: 'input', value: string): void
  (e: 'change', value: string): void
  (e: 'focus', event: FocusEvent): void
  (e: 'blur', event: FocusEvent): void
  (e: 'keydown', event: KeyboardEvent): void
}

/** Component instance types */
export type FormGroupInstance = InstanceType<typeof import('./FormGroup.vue').default>
export type InputInstance = InstanceType<typeof import('./Input.vue').default>
export type TextareaInstance = InstanceType<typeof import('./Textarea.vue').default>
