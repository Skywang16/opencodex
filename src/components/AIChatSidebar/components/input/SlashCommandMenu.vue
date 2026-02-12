<script setup lang="ts">
  import { SLASH_COMMANDS, SLASH_COMMAND_ICONS, type SlashCommand } from '@/types/slashCommand'
  import { computed, nextTick, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import McpServerList from './McpServerList.vue'

  interface Emits {
    (e: 'select', command: SlashCommand): void
    (e: 'close'): void
  }

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const searchQuery = ref('')
  const searchInput = ref<HTMLInputElement>()
  const selectedIndex = ref(0)
  const showMcpList = ref(false)

  const allCommands = computed(() => SLASH_COMMANDS)

  const filteredCommands = computed(() => {
    if (!searchQuery.value) return allCommands.value
    const query = searchQuery.value.toLowerCase()
    return allCommands.value.filter(
      cmd =>
        t(cmd.labelKey).toLowerCase().includes(query) ||
        (cmd.descriptionKey && t(cmd.descriptionKey).toLowerCase().includes(query)) ||
        cmd.id.toLowerCase().includes(query)
    )
  })

  // Organize commands by group
  const groupedCommands = computed(() => {
    const groups: { name: string | null; commands: SlashCommand[] }[] = []
    const noGroup: SlashCommand[] = []
    const groupMap = new Map<string, SlashCommand[]>()

    for (const cmd of filteredCommands.value) {
      if (cmd.group) {
        if (!groupMap.has(cmd.group)) {
          groupMap.set(cmd.group, [])
        }
        groupMap.get(cmd.group)!.push(cmd)
      } else {
        noGroup.push(cmd)
      }
    }

    if (noGroup.length > 0) {
      groups.push({ name: null, commands: noGroup })
    }

    for (const [name, commands] of groupMap) {
      groups.push({ name, commands })
    }

    return groups
  })

  // Flattened list for keyboard navigation
  const flatCommands = computed(() => {
    return groupedCommands.value.flatMap(g => g.commands)
  })

  // Reset selected index
  watch(filteredCommands, () => {
    selectedIndex.value = 0
  })

  const handleKeydown = (e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        selectedIndex.value = (selectedIndex.value + 1) % flatCommands.value.length
        break
      case 'ArrowUp':
        e.preventDefault()
        selectedIndex.value = (selectedIndex.value - 1 + flatCommands.value.length) % flatCommands.value.length
        break
      case 'Enter':
        e.preventDefault()
        if (flatCommands.value[selectedIndex.value]) {
          handleSelect(flatCommands.value[selectedIndex.value])
        }
        break
      case 'Escape':
        e.preventDefault()
        emit('close')
        break
    }
  }

  const handleSelect = (command: SlashCommand) => {
    if (command.type === 'action' && command.id === 'mcp') {
      showMcpList.value = true
      return
    }
    emit('select', command)
  }

  const handleBackFromMcp = () => {
    showMcpList.value = false
    nextTick(() => searchInput.value?.focus())
  }

  const getIcon = (iconName: string) => SLASH_COMMAND_ICONS[iconName] || ''

  onMounted(async () => {
    await nextTick()
    searchInput.value?.focus()
  })

  defineExpose({
    focus: () => searchInput.value?.focus(),
  })
</script>

<template>
  <div class="slash-command-menu" @keydown="handleKeydown">
    <McpServerList v-if="showMcpList" @back="handleBackFromMcp" />

    <template v-else>
      <div class="search-wrapper">
      <input
        ref="searchInput"
        v-model="searchQuery"
        type="text"
        class="search-input"
        :placeholder="t('slash_commands.search_placeholder')"
      />
    </div>

    <div class="commands-list">
      <template v-for="group in groupedCommands" :key="group.name || 'default'">
        <div v-if="group.name" class="group-header">
          {{ group.name }}
        </div>
        <button
          v-for="command in group.commands"
          :key="command.id"
          class="command-item"
          :class="{ selected: flatCommands.indexOf(command) === selectedIndex }"
          @click="handleSelect(command)"
          @mouseenter="selectedIndex = flatCommands.indexOf(command)"
        >
          <span class="command-icon" v-html="getIcon(command.icon)" />
          <span class="command-content">
            <span class="command-label">{{ t(command.labelKey) }}</span>
            <span v-if="command.descriptionKey" class="command-description">{{ t(command.descriptionKey) }}</span>
          </span>
          <span v-if="command.badge" class="command-badge">{{ command.badge }}</span>
        </button>
      </template>

      <div v-if="filteredCommands.length === 0" class="no-results">
        {{ t('slash_commands.no_results') }}
      </div>
    </div>
    </template>
  </div>
</template>

<style scoped>
  .slash-command-menu {
    width: 100%;
    max-height: 400px;
    display: flex;
    flex-direction: column;
  }

  .search-wrapper {
    padding: 12px 16px;
  }

  .search-input {
    width: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-200);
    font-size: 14px;
    outline: none;
    font-weight: 400;
  }

  .search-input::placeholder {
    color: var(--text-400);
  }

  .commands-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  .group-header {
    padding: 12px 10px 6px;
    font-size: 13px;
    font-weight: 400;
    color: var(--text-400);
  }

  .command-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.12s ease;
    line-height: 1.4;
  }

  .command-item:hover,
  .command-item.selected {
    background: var(--bg-300);
  }

  .command-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--text-300);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .command-icon :deep(svg) {
    width: 16px;
    height: 16px;
    display: block;
    vertical-align: middle;
  }

  .command-content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    line-height: 1.4;
  }

  .command-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-100);
    flex-shrink: 0;
    line-height: 1.4;
  }

  .command-description {
    font-size: 14px;
    font-weight: 400;
    color: var(--text-400);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
  }

  .command-badge {
    flex-shrink: 0;
    font-size: 14px;
    font-weight: 400;
    color: var(--text-400);
    line-height: 1.4;
  }

  .no-results {
    padding: 20px;
    text-align: center;
    font-size: 14px;
    color: var(--text-400);
  }

  .commands-list::-webkit-scrollbar {
    width: 4px;
  }

  .commands-list::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: var(--border-radius-xs);
  }
</style>
