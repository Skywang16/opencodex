import type { ChannelCallbacks, ChannelSubscription } from '@/api/channel'
import { channel } from '@/api/channel'
import { invoke } from '@/utils/request'

export interface VectorIndexStatus {
  isReady: boolean
  path: string
  size?: string
  sizeBytes?: number
  totalFiles: number
  totalChunks: number
  model: string
  dim: number
}

export interface VectorBuildProgress {
  phase: 'pending' | 'collecting_files' | 'chunking' | 'embedding' | 'writing' | 'completed' | 'cancelled' | 'failed'
  root: string
  totalFiles: number
  filesDone: number
  filesFailed: number
  currentFile?: string
  currentFileChunksTotal: number
  currentFileChunksDone: number
  isDone: boolean
  error?: string
}

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) return ''
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let n = bytes
  let i = 0
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i += 1
  }
  const digits = i === 0 ? 0 : n >= 100 ? 0 : n >= 10 ? 1 : 2
  return `${n.toFixed(digits)} ${units[i]}`
}

export class VectorDbApi {
  getIndexStatus = async (params: { path: string }): Promise<VectorIndexStatus> => {
    const raw = await invoke<{
      totalFiles: number
      totalChunks: number
      embeddingModel: string
      vectorDimension: number
      sizeBytes: number
    }>('get_index_status', { path: params.path })
    return {
      isReady: raw.totalChunks > 0,
      path: params.path,
      sizeBytes: raw.sizeBytes,
      size: formatBytes(raw.sizeBytes),
      totalFiles: raw.totalFiles,
      totalChunks: raw.totalChunks,
      model: raw.embeddingModel,
      dim: raw.vectorDimension,
    }
  }

  deleteWorkspaceIndex = async (path: string): Promise<void> => invoke('delete_workspace_index', { path })

  startBuildIndex = async (params: { root: string }): Promise<void> =>
    invoke('vector_build_index_start', { path: params.root })

  getBuildStatus = async (params: { root: string }): Promise<VectorBuildProgress | null> => {
    return invoke<VectorBuildProgress | null>('vector_build_index_status', { path: params.root })
  }

  subscribeBuildProgress = (
    params: { root: string },
    callbacks: ChannelCallbacks<VectorBuildProgress>
  ): ChannelSubscription => {
    return channel.subscribe<VectorBuildProgress>('vector_build_index_subscribe', { path: params.root }, callbacks, {
      cancelCommand: 'vector_build_index_cancel',
    })
  }

  cancelBuild = async (params: { root: string }): Promise<void> =>
    invoke('vector_build_index_cancel', { path: params.root })

  reloadEmbeddingConfig = async (): Promise<void> => invoke('vector_reload_embedding_config')
}

export const vectorDbApi = new VectorDbApi()
