import { invoke } from '@/utils/request'
import type {
  LspDocumentSymbol,
  LspFileDiagnostics,
  LspHoverResult,
  LspLocation,
  LspServerStatus,
  LspWorkspaceSymbol,
} from './types'

export class LspApi {
  status = async (): Promise<LspServerStatus[]> => {
    return await invoke<LspServerStatus[]>('lsp_status')
  }

  documentSymbols = async (workspace: string, path: string): Promise<LspDocumentSymbol[]> => {
    return await invoke<LspDocumentSymbol[]>('lsp_document_symbols', { workspace, path })
  }

  workspaceSymbols = async (workspace: string, query: string, pathHint?: string): Promise<LspWorkspaceSymbol[]> => {
    return await invoke<LspWorkspaceSymbol[]>('lsp_workspace_symbols', { workspace, query, pathHint: pathHint ?? null })
  }

  hover = async (workspace: string, path: string, line: number, character: number): Promise<LspHoverResult | null> => {
    return await invoke<LspHoverResult | null>('lsp_hover', { workspace, path, line, character })
  }

  definition = async (workspace: string, path: string, line: number, character: number): Promise<LspLocation[]> => {
    return await invoke<LspLocation[]>('lsp_definition', { workspace, path, line, character })
  }

  references = async (workspace: string, path: string, line: number, character: number): Promise<LspLocation[]> => {
    return await invoke<LspLocation[]>('lsp_references', { workspace, path, line, character })
  }

  diagnostics = async (workspace: string, path?: string): Promise<LspFileDiagnostics[]> => {
    return await invoke<LspFileDiagnostics[]>('lsp_diagnostics', { workspace, path: path ?? null })
  }
}

export const lspApi = new LspApi()
export type * from './types'
