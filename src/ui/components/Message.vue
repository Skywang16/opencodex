<template>
  <div v-if="visible" :class="messageClasses" role="alert" :aria-live="type === 'error' ? 'assertive' : 'polite'">
    <div class="x-message__content">
      <div v-if="dangerouslyUseHTMLString" class="x-message__text" v-html="message"></div>
      <div v-else class="x-message__text">{{ message }}</div>
    </div>
    <button v-if="closable" class="x-message__close" type="button" aria-label="Close message" @click="handleClose">
      <svg class="x-message__close-icon" viewBox="0 0 24 24">
        <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
  import { computed, inject, onMounted, onUnmounted } from 'vue'
  import type { MessageProps } from '../types/index'

  const props = withDefaults(defineProps<MessageProps>(), {
    type: 'info',
    duration: 3000,
    closable: false,
    showIcon: true,
    dangerouslyUseHTMLString: false,
  })

  const emit = defineEmits<{
    close: []
  }>()

  inject('xui-config', {})

  let timer: number | null = null

  const messageClasses = computed(() => [
    'x-message',
    `x-message--${props.type}`,
    {
      'x-message--closable': props.closable,
    },
  ])

  const handleClose = () => {
    clearTimer()
    emit('close')
  }

  const clearTimer = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
  }

  const startTimer = () => {
    if (props.duration > 0) {
      timer = window.setTimeout(() => {
        handleClose()
      }, props.duration)
    }
  }

  onMounted(() => {
    if (props.visible) {
      startTimer()
    }
  })

  onUnmounted(() => {
    clearTimer()
  })
</script>

<style scoped>
  .x-message {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.08),
      0 1px 3px rgba(0, 0, 0, 0.06);
    font-size: 13px;
    color: var(--text-200);
    white-space: nowrap;
    max-width: 480px;
  }

  /* All variants share the same clean look — no colored backgrounds */
  .x-message--success,
  .x-message--error,
  .x-message--warning,
  .x-message--info {
    /* intentionally empty — uniform style */
  }

  .x-message__content {
    flex: 1;
    min-width: 0;
  }

  .x-message__text {
    margin: 0;
    color: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .x-message__text :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
    font-weight: 500;
  }

  .x-message__text :deep(a:hover) {
    text-decoration: underline;
  }

  .x-message__close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    background: transparent;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 50%;
    flex-shrink: 0;
    transition: color 0.1s ease;
  }

  .x-message__close:hover {
    color: var(--text-200);
  }

  .x-message__close-icon {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    fill: none;
  }

  @media (max-width: 768px) {
    .x-message {
      max-width: calc(100vw - 48px);
    }
  }
</style>
