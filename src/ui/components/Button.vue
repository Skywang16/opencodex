<template>
  <button
    :class="buttonClasses"
    :disabled="disabled || loading"
    :aria-disabled="disabled || loading"
    :aria-label="ariaLabel"
    :type="type"
    @click="handleClick"
    @focus="handleFocus"
    @blur="handleBlur"
  >
    <span v-if="loading" class="x-button__loading">
      <svg class="x-button__loading-icon" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" fill="none" opacity="0.25" />
        <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />
      </svg>
    </span>

    <span v-if="showLeftIcon" class="x-button__icon x-button__icon--left">
      <slot name="icon">
        <svg v-if="icon" class="x-button__icon-svg" viewBox="0 0 24 24">
          <use :href="`#${icon}`"></use>
        </svg>
      </slot>
    </span>

    <span v-if="!circle && $slots.default" class="x-button__content">
      <slot></slot>
    </span>

    <span v-if="showRightIcon" class="x-button__icon x-button__icon--right">
      <slot name="icon">
        <svg v-if="icon" class="x-button__icon-svg" viewBox="0 0 24 24">
          <use :href="`#${icon}`"></use>
        </svg>
      </slot>
    </span>

    <!-- Link variant: auto external-link icon when no custom icon provided -->
    <svg
      v-if="variant === 'link' && !icon && !$slots.icon && $slots.default"
      class="x-button__link-arrow"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" y1="14" x2="21" y2="3" />
    </svg>
  </button>
</template>

<script setup lang="ts">
  import { computed, inject, useSlots } from 'vue'
  import type { ButtonProps } from '../types/index'

  const props = withDefaults(defineProps<ButtonProps>(), {
    variant: 'primary',
    size: 'medium',
    disabled: false,
    loading: false,
    type: 'button',
    iconPosition: 'left',
    block: false,
    round: false,
    circle: false,
  })

  const emit = defineEmits<{
    click: [event: MouseEvent]
  }>()

  const slots = useSlots()
  inject('xui-config', {})

  const buttonClasses = computed(() => [
    'x-button',
    `x-button--${props.variant}`,
    `x-button--${props.size}`,
    {
      'x-button--loading': props.loading,
      'x-button--disabled': props.disabled,
      'x-button--block': props.block,
      'x-button--round': props.round,
      'x-button--circle': props.circle,
      'x-button--icon-only': props.circle || (!slots.default && (props.icon || slots.icon)),
    },
  ])

  const showLeftIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'left'
  })

  const showRightIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'right'
  })

  const ariaLabel = computed(() => {
    if (props.loading) {
      return 'Loading'
    }
    return undefined
  })

  const handleClick = (event: MouseEvent) => {
    if (props.disabled || props.loading) {
      event.preventDefault()
      return
    }
    emit('click', event)
  }

  const handleFocus = (event: FocusEvent) => {
    void event
  }

  const handleBlur = (event: FocusEvent) => {
    void event
  }
</script>

<style>
  /* ── Design tokens ── */
  .x-button {
    --x-button-border-radius: var(
      --border-radius-xl
    ); /* 12px — visually aligned with 16px modal/input on larger surfaces */
    --x-button-transition: all 0.15s ease;

    --x-button-padding-small: 6px 14px;
    --x-button-font-size-small: 13px;

    --x-button-padding-medium: 8px 18px;
    --x-button-font-size-medium: 14px;

    --x-button-padding-large: 10px 22px;
    --x-button-font-size-large: 15px;
  }

  /* ── Base ── */
  .x-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: var(--x-button-padding-medium);
    font-size: var(--x-button-font-size-medium);
    font-family: inherit;
    font-weight: 500;
    line-height: 1.4;
    white-space: nowrap;
    text-align: center;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--x-button-border-radius);
    color: var(--text-200);
    cursor: pointer;
    user-select: none;
    touch-action: manipulation;
    outline: none;
    transition: var(--x-button-transition);
  }

  .x-button:hover {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  .x-button:focus-visible {
    box-shadow: 0 0 0 3px var(--color-primary-alpha);
    outline: none;
  }

  .x-button:active {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  /* ── Sizes ── */
  .x-button--small {
    padding: var(--x-button-padding-small);
    font-size: var(--x-button-font-size-small);
  }

  .x-button--large {
    padding: var(--x-button-padding-large);
    font-size: var(--x-button-font-size-large);
  }

  /* ── Primary: solid filled (like "Try" in reference) ── */
  .x-button--primary {
    color: #fff;
    background: var(--text-100);
    border-color: var(--text-100);
    font-weight: 600;
  }

  .x-button--primary:hover {
    color: #fff;
    background: var(--text-200);
    border-color: var(--text-200);
  }

  .x-button--primary:active {
    color: #fff;
    background: var(--text-100);
    border-color: var(--text-100);
  }

  /* ── Secondary: light neutral fill (like "Open" in reference) ── */
  .x-button--secondary {
    color: var(--text-200);
    background: var(--bg-200);
    border-color: var(--border-200);
  }

  .x-button--secondary:hover {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  .x-button--secondary:active {
    background: var(--bg-300);
    border-color: var(--border-300);
  }

  /* ── Danger: like "Uninstall" in reference — transparent bg, red text ── */
  .x-button--danger {
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    border-color: color-mix(in srgb, var(--color-error) 15%, transparent);
    font-weight: 500;
  }

  .x-button--danger:hover {
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 14%, transparent);
    border-color: color-mix(in srgb, var(--color-error) 25%, transparent);
  }

  .x-button--danger:active {
    background: color-mix(in srgb, var(--color-error) 18%, transparent);
    border-color: color-mix(in srgb, var(--color-error) 30%, transparent);
  }

  .x-button--danger:focus-visible {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-error) 15%, transparent);
  }

  /* ── Ghost: transparent with border ── */
  .x-button--ghost {
    color: var(--text-200);
    background: transparent;
    border-color: var(--border-200);
  }

  .x-button--ghost:hover {
    background: var(--bg-200);
    border-color: var(--border-300);
  }

  .x-button--ghost:active {
    background: var(--bg-300);
  }

  /* ── Link: text + auto external icon ── */
  .x-button--link {
    color: var(--color-primary);
    background: transparent;
    border-color: transparent;
    padding: 0 4px;
    gap: 4px;
  }

  .x-button--link:hover {
    color: var(--color-primary-hover);
    background: transparent;
    border-color: transparent;
  }

  .x-button--link:active {
    color: var(--color-primary-hover);
  }

  .x-button__link-arrow {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    opacity: 0.7;
  }

  /* ── Disabled ── */
  .x-button--disabled,
  .x-button:disabled {
    color: var(--text-500) !important;
    background: var(--bg-200) !important;
    border-color: var(--border-200) !important;
    cursor: not-allowed !important;
    opacity: 0.5 !important;
  }

  .x-button--link.x-button--disabled,
  .x-button--link:disabled {
    background: transparent !important;
    border-color: transparent !important;
  }

  /* ── Loading ── */
  .x-button--loading {
    pointer-events: none;
    cursor: default;
  }

  .x-button--loading .x-button__content {
    opacity: 0.7;
  }

  /* ── Block ── */
  .x-button--block {
    width: 100%;
  }

  /* ── Round ── */
  .x-button--round {
    border-radius: 999px;
  }

  /* ── Circle ── */
  .x-button--circle {
    width: 36px;
    height: 36px;
    min-width: 36px;
    padding: 0;
    border-radius: 50%;
  }

  .x-button--circle.x-button--small {
    width: 28px;
    height: 28px;
    min-width: 28px;
  }

  .x-button--circle.x-button--large {
    width: 44px;
    height: 44px;
    min-width: 44px;
  }

  /* ── Icon-only ── */
  .x-button--icon-only {
    padding: 8px;
    min-width: 36px;
  }

  .x-button--icon-only.x-button--small {
    padding: 6px;
    min-width: 28px;
  }

  .x-button--icon-only.x-button--large {
    padding: 10px;
    min-width: 44px;
  }

  /* ── Internal elements ── */
  .x-button__loading {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  .x-button__loading-icon {
    width: 14px;
    height: 14px;
    animation: x-button-spin 1s linear infinite;
  }

  .x-button--small .x-button__loading-icon {
    width: 12px;
    height: 12px;
  }

  .x-button--large .x-button__loading-icon {
    width: 16px;
    height: 16px;
  }

  .x-button__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  .x-button__icon :slotted(svg) {
    width: 14px;
    height: 14px;
  }

  .x-button__icon-svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }

  .x-button--small .x-button__icon-svg,
  .x-button--small .x-button__icon :slotted(svg) {
    width: 12px;
    height: 12px;
  }

  .x-button--large .x-button__icon-svg,
  .x-button--large .x-button__icon :slotted(svg) {
    width: 16px;
    height: 16px;
  }

  .x-button__content {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  @keyframes x-button-spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  @media (prefers-contrast: high) {
    .x-button {
      border-width: 2px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .x-button {
      transition: none;
    }
    .x-button__loading-icon {
      animation: none;
    }
  }
</style>
