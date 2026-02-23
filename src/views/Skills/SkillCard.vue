<script setup lang="ts">
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()

  defineProps<{
    name: string
    description: string
    color: string
    initial: string
    /** 'installed' shows source badge, 'discover' shows install button */
    variant: 'installed' | 'discover'
    /** Badge text for installed skills (e.g. "Global", workspace dir name) */
    source?: string
  }>()

  defineEmits<{
    (e: 'click'): void
    (e: 'quick-install'): void
  }>()
</script>

<template>
  <div class="skill-card" :class="{ 'discover-card': variant === 'discover' }" @click="$emit('click')">
    <div class="skill-card-header">
      <div class="skill-icon-wrapper" :style="{ background: color + '18', color }">
        <span class="skill-initial">{{ initial }}</span>
      </div>
      <div class="skill-info">
        <h3 class="skill-name">{{ name }}</h3>
        <p class="skill-desc">{{ description }}</p>
      </div>
      <span v-if="variant === 'installed' && source" class="skill-source-badge">
        {{ source }}
      </span>
      <div v-if="variant === 'discover'" class="skill-actions">
        <button class="quick-install-btn" :title="t('skills_page.install')" @click.stop="$emit('quick-install')">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M12 5v14m7-7H5" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .skill-card {
    display: flex;
    flex-direction: column;
    padding: 16px;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-2xl);
    cursor: pointer;
  }

  .skill-card:hover {
    background: var(--color-hover);
    border-color: var(--border-300);
  }

  .skill-card-header {
    display: flex;
    align-items: center;
    width: 100%;
  }

  .skill-icon-wrapper {
    width: 44px;
    height: 44px;
    border-radius: var(--border-radius-xl);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-right: 14px;
    flex-shrink: 0;
    background: var(--bg-200);
    color: var(--text-400);
  }

  .skill-initial {
    font-size: 18px;
    font-weight: 700;
    line-height: 1;
  }

  .skill-info {
    flex: 1;
    min-width: 0;
  }

  .skill-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
    margin-bottom: 2px;
  }

  .skill-desc {
    font-size: 13px;
    color: var(--text-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
  }

  .skill-source-badge {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-400);
    background: var(--bg-200);
    padding: 4px 8px;
    border-radius: var(--border-radius-md);
    flex-shrink: 0;
    margin-left: 12px;
  }

  .skill-actions {
    flex-shrink: 0;
    margin-left: 12px;
  }

  .quick-install-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    cursor: pointer;
  }

  .quick-install-btn:hover {
    background: var(--bg-200);
    border-color: var(--border-300);
    color: var(--text-200);
  }

  .quick-install-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
