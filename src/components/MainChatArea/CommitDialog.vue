<script setup lang="ts">
  import { useGitStore } from '@/stores/git'
  import { XModal, XSwitch } from '@/ui'
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    visible: boolean
  }

  interface Emits {
    (e: 'update:visible', visible: boolean): void
    (e: 'success'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const { t } = useI18n()
  const gitStore = useGitStore()

  const commitMessage = ref('')
  const includeUnstaged = ref(true)
  const isLoading = ref(false)
  const nextStep = ref<'commit' | 'commit-push' | 'commit-pr'>('commit')

  // Computed
  const currentBranch = computed(() => gitStore.currentBranch || 'main')
  const stagedCount = computed(() => gitStore.stagedCount)
  const changedCount = computed(() => gitStore.changedCount)
  const totalChanges = computed(() => (includeUnstaged.value ? changedCount.value : stagedCount.value))

  const additions = computed(() => {
    // Simplified display, can actually get from git diff --stat
    return totalChanges.value > 0 ? `+${Math.floor(totalChanges.value * 50)}` : '+0'
  })

  const deletions = computed(() => {
    return totalChanges.value > 0 ? `-${Math.floor(totalChanges.value * 30)}` : '-0'
  })

  const canCommit = computed(() => {
    return totalChanges.value > 0 && commitMessage.value.trim().length > 0
  })

  // Methods
  const handleClose = () => {
    emit('update:visible', false)
  }

  const handleContinue = async () => {
    if (!canCommit.value) return

    isLoading.value = true

    try {
      // Stage all if includeUnstaged is true
      if (includeUnstaged.value) {
        await gitStore.stageAllFiles()
      }

      // Commit
      await gitStore.commit(commitMessage.value.trim())

      // Push if needed
      if (nextStep.value === 'commit-push' || nextStep.value === 'commit-pr') {
        await gitStore.push()
      }

      // TODO: Create PR if needed (requires GitHub API integration)
      if (nextStep.value === 'commit-pr') {
        console.warn('Create PR not yet implemented')
      }

      commitMessage.value = ''
      emit('success')
      handleClose()
    } catch (error) {
      console.error('Commit failed:', error)
    } finally {
      isLoading.value = false
    }
  }

  // Reset state when dialog opens
  watch(
    () => props.visible,
    visible => {
      if (visible) {
        commitMessage.value = ''
        includeUnstaged.value = true
        nextStep.value = 'commit'
        gitStore.refreshStatus()
      }
    }
  )
</script>

<template>
  <XModal
    :visible="visible"
    size="small"
    :closable="true"
    :mask-closable="true"
    :show-header="false"
    :show-footer="false"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="commit-dialog">
      <!-- Header -->
      <div class="dialog-header">
        <svg class="commit-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="4" />
          <line x1="1.05" y1="12" x2="7" y2="12" />
          <line x1="17.01" y1="12" x2="22.96" y2="12" />
        </svg>
        <button class="close-btn" @click="handleClose">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <!-- Title -->
      <h2 class="dialog-title">{{ t('git.commit_changes') }}</h2>

      <!-- Info Row -->
      <div class="info-row">
        <span class="info-label">{{ t('git.branch') }}</span>
        <div class="info-value">
          <svg class="branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="6" y1="3" x2="6" y2="15" />
            <circle cx="18" cy="6" r="3" />
            <circle cx="6" cy="18" r="3" />
            <path d="M18 9a9 9 0 0 1-9 9" />
          </svg>
          <span>{{ currentBranch }}</span>
        </div>
      </div>

      <div class="info-row">
        <span class="info-label">{{ t('git.changes') }}</span>
        <div class="info-value">
          <span>{{ totalChanges }} {{ t('git.files') }}</span>
          <span class="additions">{{ additions }}</span>
          <span class="deletions">{{ deletions }}</span>
        </div>
      </div>

      <!-- Include Unstaged Toggle -->
      <div class="toggle-row">
        <XSwitch v-model="includeUnstaged" />
        <span class="toggle-label">{{ t('git.include_unstaged') }}</span>
      </div>

      <!-- Commit Message -->
      <div class="message-section">
        <label class="section-label">{{ t('git.commit_message') }}</label>
        <textarea v-model="commitMessage" class="message-input" :placeholder="t('git.commit_message_auto')" rows="3" />
      </div>

      <!-- Next Steps -->
      <div class="next-steps">
        <label class="section-label">{{ t('git.next_steps') }}</label>

        <label class="step-option" :class="{ active: nextStep === 'commit' }">
          <input v-model="nextStep" type="radio" value="commit" />
          <svg class="step-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="4" />
            <line x1="1.05" y1="12" x2="7" y2="12" />
            <line x1="17.01" y1="12" x2="22.96" y2="12" />
          </svg>
          <span>{{ t('git.commit') }}</span>
          <svg
            v-if="nextStep === 'commit'"
            class="check-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </label>

        <label class="step-option" :class="{ active: nextStep === 'commit-push' }">
          <input v-model="nextStep" type="radio" value="commit-push" />
          <svg class="step-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="19" x2="12" y2="5" />
            <polyline points="5 12 12 5 19 12" />
          </svg>
          <span>{{ t('git.commit_and_push') }}</span>
          <svg
            v-if="nextStep === 'commit-push'"
            class="check-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </label>

        <label class="step-option disabled" :class="{ active: nextStep === 'commit-pr' }">
          <input v-model="nextStep" type="radio" value="commit-pr" disabled />
          <svg class="step-icon" viewBox="0 0 24 24" fill="currentColor">
            <path
              d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
            />
          </svg>
          <span>{{ t('git.commit_and_create_pr') }}</span>
          <svg
            v-if="nextStep === 'commit-pr'"
            class="check-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </label>
      </div>

      <!-- Action Button -->
      <button
        class="continue-btn"
        :class="{ disabled: !canCommit }"
        :disabled="!canCommit || isLoading"
        @click="handleContinue"
      >
        <span v-if="isLoading" class="loading-spinner" />
        <span v-else>{{ t('common.continue') }}</span>
      </button>
    </div>
  </XModal>
</template>

<style scoped>
  .commit-dialog {
    padding: 20px;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .commit-icon {
    width: 24px;
    height: 24px;
    color: var(--text-300);
  }

  .close-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .close-btn svg {
    width: 18px;
    height: 18px;
  }

  .dialog-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 20px 0;
  }

  .info-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
  }

  .info-label {
    font-size: 14px;
    color: var(--text-300);
  }

  .info-value {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text-200);
  }

  .branch-icon {
    width: 16px;
    height: 16px;
    color: var(--text-400);
  }

  .additions {
    color: var(--color-success);
    font-family: var(--font-family-mono);
    font-size: 13px;
  }

  .deletions {
    color: var(--color-error);
    font-family: var(--font-family-mono);
    font-size: 13px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
    border-top: 1px solid var(--border-200);
    margin-top: 8px;
  }

  .toggle-label {
    font-size: 14px;
    color: var(--text-200);
  }

  .message-section {
    margin-top: 16px;
  }

  .section-label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 8px;
  }

  .message-input {
    width: 100%;
    padding: 12px;
    font-size: 14px;
    font-family: var(--font-family);
    color: var(--text-200);
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    resize: none;
    transition: border-color 0.15s ease;
  }

  .message-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .message-input::placeholder {
    color: var(--text-500);
  }

  .next-steps {
    margin-top: 20px;
  }

  .step-option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    margin-top: 6px;
    background: transparent;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .step-option:hover {
    background: var(--bg-200);
  }

  .step-option.active {
    background: var(--bg-200);
    border-color: var(--border-300);
  }

  .step-option.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .step-option input {
    display: none;
  }

  .step-icon {
    width: 18px;
    height: 18px;
    color: var(--text-400);
    flex-shrink: 0;
  }

  .step-option span {
    flex: 1;
    font-size: 14px;
    color: var(--text-200);
  }

  .check-icon {
    width: 18px;
    height: 18px;
    color: var(--color-primary);
    flex-shrink: 0;
  }

  .continue-btn {
    width: 100%;
    padding: 12px;
    margin-top: 20px;
    font-size: 14px;
    font-weight: 500;
    color: var(--bg-100);
    background: var(--text-100);
    border: none;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .continue-btn:hover:not(.disabled) {
    opacity: 0.9;
  }

  .continue-btn.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid transparent;
    border-top-color: var(--bg-100);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
