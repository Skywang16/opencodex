export interface LspServerStatus {
  serverId: string
  root: string
  command: string
  initialized: boolean
  connected: boolean
  openDocuments: number
  diagnosticsFiles: number
  lastError: string | null
}

export interface LspPosition {
  line: number
  character: number
}

export interface LspRange {
  start: LspPosition
  end: LspPosition
}

export interface LspLocation {
  path: string
  range: LspRange
}

export interface LspDocumentSymbol {
  name: string
  kind: number
  detail?: string | null
  range: LspRange
  selectionRange: LspRange
  children: LspDocumentSymbol[]
}

export interface LspWorkspaceSymbol {
  name: string
  kind: number
  containerName?: string | null
  location?: LspLocation | null
}

export interface LspHoverResult {
  contents: string
  range?: LspRange | null
}

export interface LspDiagnostic {
  range: LspRange
  severity?: number
  code?: string | number
  source?: string
  message: string
}

export interface LspFileDiagnostics {
  path: string
  diagnostics: LspDiagnostic[]
}
