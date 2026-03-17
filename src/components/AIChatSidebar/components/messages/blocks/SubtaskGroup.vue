<script setup lang="ts">
  import type { Block } from '@/types'
  import SubtaskBlock from './SubtaskBlock.vue'

  type Subtask = Extract<Block, { type: 'subtask' }>

  interface Props {
    blocks: Subtask[]
  }

  defineProps<Props>()
</script>

<template>
  <section class="subtask-group step-block">
    <div class="subtask-group-header">
      <span class="subtask-group-kicker">Parallel Agents</span>
      <span class="subtask-group-count">{{ blocks.length }} active agents</span>
    </div>

    <div class="subtask-grid">
      <SubtaskBlock v-for="block in blocks" :key="block.id" :block="block" />
    </div>
  </section>
</template>

<style scoped>
  .subtask-group {
    margin-bottom: var(--spacing-md);
    padding: 14px;
    border-radius: 20px;
    border: 1px solid color-mix(in srgb, var(--border-200) 85%, transparent);
    background: transparent;
    box-shadow: none;
  }

  .subtask-group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .subtask-group-kicker {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-500);
  }

  .subtask-group-count {
    font-size: 12px;
    color: var(--text-400);
  }

  .subtask-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
  }

  @media (max-width: 720px) {
    .subtask-group {
      padding: 12px;
      border-radius: 18px;
    }

    .subtask-grid {
      grid-template-columns: 1fr;
      gap: 10px;
    }
  }
</style>
