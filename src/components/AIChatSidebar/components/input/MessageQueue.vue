<script setup lang="ts">
  import type { QueuedMessage } from '@/components/AIChatSidebar/store'
  import { nextTick, onBeforeUnmount, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    queue: QueuedMessage[]
  }

  interface Emits {
    (e: 'remove', id: string): void
    (e: 'update', id: string, content: string): void
    (e: 'send-now', id: string): void
    (e: 'reorder', fromIndex: number, toIndex: number): void
  }

  defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const editingId = ref<string | null>(null)
  const editContent = ref('')
  const editInput = ref<HTMLTextAreaElement>()

  const startEdit = (msg: QueuedMessage) => {
    editingId.value = msg.id
    editContent.value = msg.content
    nextTick(() => {
      editInput.value?.focus()
      editInput.value?.select()
    })
  }

  const confirmEdit = () => {
    if (editingId.value && editContent.value.trim()) {
      emit('update', editingId.value, editContent.value.trim())
    }
    editingId.value = null
  }

  const cancelEdit = () => {
    editingId.value = null
  }

  const handleEditKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      confirmEdit()
    } else if (event.key === 'Escape') {
      cancelEdit()
    }
  }

  // Pointer-based drag reorder
  const dragging = ref(false)
  const dragFromIndex = ref(-1)
  const hoverIndex = ref(-1)
  const listEl = ref<HTMLElement>()

  let itemRects: DOMRect[] = []

  const onGripPointerDown = (index: number, event: PointerEvent) => {
    event.preventDefault()
    dragging.value = true
    dragFromIndex.value = index
    hoverIndex.value = index

    // Snapshot item positions
    if (listEl.value) {
      const items = listEl.value.querySelectorAll('.queue-item')
      itemRects = Array.from(items).map(el => el.getBoundingClientRect())
    }

    document.addEventListener('pointermove', onPointerMove)
    document.addEventListener('pointerup', onPointerUp)
  }

  const onPointerMove = (event: PointerEvent) => {
    if (!dragging.value) return
    const y = event.clientY
    // Find which item the pointer is over
    for (let i = 0; i < itemRects.length; i++) {
      const rect = itemRects[i]
      const midY = rect.top + rect.height / 2
      if (y < midY) {
        hoverIndex.value = i
        return
      }
    }
    hoverIndex.value = itemRects.length - 1
  }

  const onPointerUp = () => {
    if (dragging.value && dragFromIndex.value !== hoverIndex.value && hoverIndex.value >= 0) {
      emit('reorder', dragFromIndex.value, hoverIndex.value)
    }
    dragging.value = false
    dragFromIndex.value = -1
    hoverIndex.value = -1
    itemRects = []
    document.removeEventListener('pointermove', onPointerMove)
    document.removeEventListener('pointerup', onPointerUp)
  }

  onBeforeUnmount(() => {
    document.removeEventListener('pointermove', onPointerMove)
    document.removeEventListener('pointerup', onPointerUp)
  })

  const getItemTransform = (index: number): string => {
    if (!dragging.value || dragFromIndex.value < 0 || hoverIndex.value < 0) return ''
    if (dragFromIndex.value === hoverIndex.value) return ''
    const from = dragFromIndex.value
    const to = hoverIndex.value
    if (index === from) {
      // Dragged item: shift to target position
      const dy = itemRects[to] ? itemRects[to].top - itemRects[from].top : 0
      return `translateY(${dy}px)`
    }
    if (from < to && index > from && index <= to) {
      // Items between from..to shift up by one item height
      const dy = itemRects[from] ? -(itemRects[from].height + 2) : 0
      return `translateY(${dy}px)`
    }
    if (from > to && index >= to && index < from) {
      // Items between to..from shift down by one item height
      const dy = itemRects[from] ? itemRects[from].height + 2 : 0
      return `translateY(${dy}px)`
    }
    return ''
  }
</script>

<template>
  <div v-if="queue.length > 0" class="message-queue">
    <div class="queue-header">
      <span class="queue-count">{{ t('chat.queue_count', { count: queue.length }) }}</span>
    </div>
    <div ref="listEl" class="queue-list">
      <div
        v-for="(msg, index) in queue"
        :key="msg.id"
        class="queue-item"
        :class="{
          'queue-item--active': dragging && dragFromIndex === index,
          'queue-item--shifting': dragging && dragFromIndex !== index,
        }"
        :style="{ transform: getItemTransform(index) }"
      >
        <span class="queue-grip" @pointerdown="onGripPointerDown(index, $event)">
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="5" cy="4" r="1.5" />
            <circle cx="11" cy="4" r="1.5" />
            <circle cx="5" cy="10" r="1.5" />
            <circle cx="11" cy="10" r="1.5" />
          </svg>
        </span>

        <!-- Editing mode -->
        <div v-if="editingId === msg.id" class="queue-edit">
          <textarea
            ref="editInput"
            v-model="editContent"
            class="queue-edit-input"
            rows="2"
            @keydown="handleEditKeydown"
            @blur="cancelEdit"
          />
        </div>

        <!-- Display mode -->
        <span v-else class="queue-content" :title="msg.content">{{ msg.content }}</span>

        <div class="queue-actions">
          <button class="queue-action-btn" :title="t('chat.queue_edit')" @click.stop="startEdit(msg)">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
              <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
          </button>
          <button class="queue-action-btn" :title="t('chat.queue_delete')" @click.stop="emit('remove', msg.id)">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
          </button>
          <button class="queue-action-btn" :title="t('chat.queue_send_now')" @click.stop="emit('send-now', msg.id)">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="m3 3 3 9-3 9 19-9z" />
              <path d="m6 12h16" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .message-queue {
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border-200);
    margin-bottom: 10px;
  }

  .queue-header {
    padding: 0 0 6px;
  }

  .queue-count {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-400);
  }

  .queue-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: relative;
  }

  .queue-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: var(--border-radius-md);
    transition:
      background 0.12s ease,
      transform 0.2s cubic-bezier(0.2, 0, 0, 1);
    user-select: none;
  }

  .queue-item:hover {
    background: var(--bg-200);
  }

  .queue-item--active {
    z-index: 10;
    background: var(--bg-200);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    transition: box-shadow 0.15s ease;
  }

  .queue-item--shifting {
    pointer-events: none;
  }

  .queue-grip {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    color: var(--text-400);
    opacity: 0.4;
    cursor: grab;
    touch-action: none;
  }

  .queue-grip:active {
    cursor: grabbing;
    opacity: 0.7;
  }

  .queue-content {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .queue-edit {
    flex: 1;
    min-width: 0;
  }

  .queue-edit-input {
    width: 100%;
    padding: 2px 6px;
    font-size: 13px;
    color: var(--text-200);
    background: var(--bg-100);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    outline: none;
    resize: none;
    font-family: inherit;
    line-height: 1.4;
  }

  .queue-edit-input:focus {
    border-color: var(--color-primary);
  }

  .queue-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 1px;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .queue-item:hover .queue-actions {
    opacity: 1;
  }

  .queue-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .queue-action-btn:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }
</style>
