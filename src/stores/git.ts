import { gitApi } from '@/api'
import type { BranchInfo, CommitInfo, FileChange, RepositoryStatus } from '@/api/git/types'
import { useFileWatcherStore } from '@/stores/fileWatcher'
import { useWorkspaceStore } from '@/stores/workspace'
import { debounce } from 'lodash-es'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

export const useGitStore = defineStore('git', () => {
  const fileWatcherStore = useFileWatcherStore()

  const status = ref<RepositoryStatus | null>(null)
  const branches = ref<BranchInfo[]>([])
  const commits = ref<CommitInfo[]>([])
  const commitsHasMore = ref(true)
  let commitsInFlight: Promise<void> | null = null
  const selectedFile = ref<FileChange | null>(null)
  const selectedFileIsStaged = ref(false)

  // Diff statistics (line additions/deletions)
  const diffAdditions = ref(0)
  const diffDeletions = ref(0)

  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const isRepository = computed(() => status.value?.isRepository ?? false)
  const repositoryRoot = computed(() => status.value?.rootPath ?? null)
  const currentBranch = computed(() => status.value?.currentBranch ?? null)
  const stagedCount = computed(() => status.value?.stagedFiles.length ?? 0)
  const changedCount = computed(() => {
    const staged = status.value?.stagedFiles.length ?? 0
    const modified = status.value?.modifiedFiles.length ?? 0
    const untracked = status.value?.untrackedFiles.length ?? 0
    const conflicted = status.value?.conflictedFiles.length ?? 0
    return staged + modified + untracked + conflicted
  })

  const currentPath = computed(() => {
    return useWorkspaceStore().currentWorkspacePath ?? null
  })

  let pendingRefresh = false
  const refreshStatus = async () => {
    const path = currentPath.value
    if (!path || path === '~') {
      status.value = null
      branches.value = []
      commits.value = []
      selectedFile.value = null
      diffAdditions.value = 0
      diffDeletions.value = 0
      return
    }

    if (isLoading.value) {
      pendingRefresh = true
      return
    }

    isLoading.value = true
    error.value = null

    try {
      // Use silent mode to avoid showing "Git not found" errors
      status.value = await gitApi.getStatus(path, true)

      if (!status.value.isRepository) {
        branches.value = []
        commits.value = []
        selectedFile.value = null
        diffAdditions.value = 0
        diffDeletions.value = 0
      } else {
        // Fetch diff statistics (silent mode)
        const stat = await gitApi.getDiffStat(path, true)
        diffAdditions.value = stat.additions
        diffDeletions.value = stat.deletions
      }
    } catch (e) {
      // Silently handle all Git-related errors
      status.value = null
      diffAdditions.value = 0
      diffDeletions.value = 0
    } finally {
      isLoading.value = false
      if (pendingRefresh) {
        pendingRefresh = false
        void refreshStatus()
      }
    }
  }

  const loadBranches = async () => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return

    try {
      branches.value = await gitApi.getBranches(path)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  const loadCommits = async (limit = 50) => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return

    while (commitsInFlight) {
      try {
        await commitsInFlight
      } catch {
        break
      }
    }

    const requestLimit = Math.min(limit + 1, 200)

    const task = (async () => {
      const fetched = await gitApi.getCommits(path, requestLimit, 0)
      commitsHasMore.value = fetched.length > limit
      commits.value = fetched.slice(0, limit)
      return commits.value.length
    })()
    commitsInFlight = task.then(
      () => {},
      () => {}
    )

    try {
      return await task
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      return 0
    } finally {
      commitsInFlight = null
    }
  }

  const loadMoreCommits = async (limit = 50): Promise<{ loaded: number; hasMore: boolean }> => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return { loaded: 0, hasMore: false }

    while (commitsInFlight) {
      try {
        await commitsInFlight
      } catch {
        break
      }
    }

    const skip = commits.value.length
    const requestLimit = Math.min(limit + 1, 200)
    const hasOverfetch = requestLimit > limit

    const task = (async () => {
      const fetched = await gitApi.getCommits(path, requestLimit, skip)
      const fetchedHasMore = hasOverfetch ? fetched.length > limit : fetched.length === limit
      const page = fetched.slice(0, limit)

      if (page.length === 0) {
        commitsHasMore.value = false
        return { loaded: 0, hasMore: false }
      }

      const existing = new Set(commits.value.map(c => c.hash))
      const unique = page.filter(c => !existing.has(c.hash))
      if (unique.length === 0) {
        commitsHasMore.value = false
        return { loaded: 0, hasMore: false }
      }

      commits.value = [...commits.value, ...unique]
      commitsHasMore.value = fetchedHasMore
      return { loaded: unique.length, hasMore: commitsHasMore.value }
    })()
    commitsInFlight = task.then(
      () => {},
      () => {}
    )

    try {
      return await task
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      return { loaded: 0, hasMore: false }
    } finally {
      commitsInFlight = null
    }
  }

  const requireRepoPath = (): string | null => {
    const path = currentPath.value
    if (!path || path === '~') return null
    return path
  }

  const toRootPathSpec = (filePath: string) => {
    if (!filePath) return filePath
    const normalized = filePath.replace(/^\/+/, '')
    return normalized.startsWith(':/') ? normalized : `:/${normalized}`
  }

  const toWorktreePathSpec = (filePath: string) => {
    if (!filePath) return filePath
    const withoutRootMagic = filePath.startsWith(':/') ? filePath.slice(2) : filePath
    return withoutRootMagic.replace(/^\/+/, '')
  }

  const stageFile = async (file: FileChange, repoPathOverride?: string) => {
    const pathSpec = toRootPathSpec(file.path)
    const path = repoPathOverride ?? requireRepoPath()
    if (!path) return
    await gitApi.stagePaths(path, [pathSpec])
    await refreshStatus()
  }

  const stageFiles = async (files: FileChange[]) => {
    if (files.length === 0) return
    const path = requireRepoPath()
    if (!path) return
    const pathSpecs = files.map(f => toRootPathSpec(f.path))
    await gitApi.stagePaths(path, pathSpecs)
    await refreshStatus()
  }

  const stageAllFiles = async () => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.stageAll(path)
    await refreshStatus()
  }

  const unstageFile = async (file: FileChange, repoPathOverride?: string) => {
    // git reset -- <path> unstage single file
    const pathSpec = toRootPathSpec(file.path)
    const path = repoPathOverride ?? requireRepoPath()
    if (!path) return
    await gitApi.unstagePaths(path, [pathSpec])
    await refreshStatus()
  }

  const unstageFiles = async (files: FileChange[]) => {
    if (files.length === 0) return
    const path = requireRepoPath()
    if (!path) return
    const pathSpecs = files.map(f => toRootPathSpec(f.path))
    await gitApi.unstagePaths(path, pathSpecs)
    await refreshStatus()
  }

  const unstageAllFiles = async () => {
    // git reset unstage all
    const path = requireRepoPath()
    if (!path) return
    await gitApi.unstageAll(path)
    await refreshStatus()
  }

  const discardFile = async (file: FileChange) => {
    const path = requireRepoPath()
    if (!path) return
    if (file.status === 'untracked') {
      // Untracked files: delete directly
      const pathSpec = toRootPathSpec(file.path)
      await gitApi.cleanPaths(path, [pathSpec])
    } else {
      // Tracked files: only discard worktree changes, keep index (avoid losing staged content)
      const pathSpec = toWorktreePathSpec(file.path)
      await gitApi.discardWorktreePaths(path, [pathSpec])
    }
    await refreshStatus()
  }

  const discardFiles = async (files: FileChange[]) => {
    if (files.length === 0) return

    const path = requireRepoPath()
    if (!path) return

    // Handle untracked and tracked files separately
    const untracked = files.filter(f => f.status === 'untracked')
    const tracked = files.filter(f => f.status !== 'untracked')

    // Restore tracked files
    if (tracked.length > 0) {
      const pathSpecs = tracked.map(f => toWorktreePathSpec(f.path))
      await gitApi.discardWorktreePaths(path, pathSpecs)
    }

    // Delete untracked files
    if (untracked.length > 0) {
      const pathSpecs = untracked.map(f => toRootPathSpec(f.path))
      await gitApi.cleanPaths(path, pathSpecs)
    }

    await refreshStatus()
  }

  const discardAllChanges = async () => {
    // Restore worktree changes for all tracked files (keep index)
    const path = requireRepoPath()
    if (!path) return

    await gitApi.discardWorktreeAll(path)
    await gitApi.cleanAll(path)
    await refreshStatus()
  }

  const commit = async (message: string) => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.commit(path, message)
    await refreshStatus()
    await loadCommits()
  }

  const push = async () => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.push(path)
    await refreshStatus()
  }

  const pull = async () => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.pull(path)
    await refreshStatus()
  }

  const fetch = async () => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.fetch(path)
    await refreshStatus()
  }

  const sync = async () => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.pull(path)
    await gitApi.push(path)
    await refreshStatus()
  }

  const checkoutBranch = async (branchName: string) => {
    const path = requireRepoPath()
    if (!path) return
    await gitApi.checkoutBranch(path, branchName)
    await refreshStatus()
    await loadBranches()
    await loadCommits()
  }

  const initRepository = async () => {
    const path = currentPath.value
    if (!path || path === '~') return

    await gitApi.initRepo(path)
    await refreshStatus()
  }

  const selectDiffFile = (file: FileChange, staged: boolean) => {
    selectedFile.value = file
    selectedFileIsStaged.value = staged
  }

  const selectCommitFile = (hash: string, filePath: string) => {
    // TODO: Implement commit file diff viewer
    console.warn('Commit file diff not yet implemented:', hash, filePath)
  }

  watch(
    currentPath,
    () => {
      void refreshStatus()
      selectedFile.value = null
      selectedFileIsStaged.value = false
    },
    { immediate: true }
  )

  let isGitChangedListenerSetup = false
  const debouncedRefreshStatus = debounce(() => {
    void refreshStatus()
  }, 150)

  const setupGitChangedListener = () => {
    if (isGitChangedListenerSetup) return
    isGitChangedListenerSetup = true

    fileWatcherStore.subscribe(batch => {
      const root = repositoryRoot.value
      if (!root) return

      for (const evt of batch.events) {
        if (evt.type !== 'git_changed') continue
        if (evt.repoRoot !== root) continue
        debouncedRefreshStatus()
        break
      }
    })
  }

  setupGitChangedListener()

  return {
    status,
    branches,
    commits,
    selectedFile,
    selectedFileIsStaged,
    isLoading,
    error,
    diffAdditions,
    diffDeletions,

    isRepository,
    repositoryRoot,
    currentBranch,
    stagedCount,
    changedCount,
    currentPath,
    commitsHasMore,

    refreshStatus,
    loadBranches,
    loadCommits,
    loadMoreCommits,

    stageFile,
    stageFiles,
    stageAllFiles,
    unstageFile,
    unstageFiles,
    unstageAllFiles,
    discardFile,
    discardFiles,
    discardAllChanges,
    commit,
    push,
    pull,
    fetch,
    sync,
    initRepository,

    selectDiffFile,
    selectCommitFile,
    checkoutBranch,
  }
})
