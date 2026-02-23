/**
 * Markdown renderer singleton
 */

import hljs from 'highlight.js'
import { marked } from 'marked'
import { markedHighlight } from 'marked-highlight'

// Configure marked (executed only once)
marked.use(
  markedHighlight({
    langPrefix: 'hljs language-',
    highlight(code, lang) {
      const language = hljs.getLanguage(lang) ? lang : 'plaintext'
      return hljs.highlight(code, { language }).value
    },
  })
)

// Configure marked rendering options
const renderer = new marked.Renderer()

// Custom code block rendering with header structure
renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
  const language = lang || 'text'
  // At this point, text is already highlighted HTML from marked-highlight (if configured)
  // Note: if we override renderer.code, we need to ensure highlighting still works
  // Actually, marked-highlight modifies tokens via hooks, so the text passed here is already highlighted HTML

  return `
      <div class="code-block-wrapper">
        <div class="code-block-header">
        <span class="code-lang">${language}</span>
        <button class="code-copy-btn" aria-label="Copy code">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
      <pre><code class="hljs language-${language}">${text}</code></pre>
    </div>
  `
}

// Custom table rendering with wrapper container for scrolling and styling
// In marked v16+, token.header/rows are object arrays, not HTML strings.
// We delegate to the prototype method with correct `this` so internal
// tablerow/tablecell calls resolve properly.
const origTable = marked.Renderer.prototype.table
// eslint-disable-next-line @typescript-eslint/no-explicit-any
renderer.table = function (this: any, token: any) {
  const inner = origTable.call(this, token)
  return `<div class="table-wrapper">${inner}</div>`
}

marked.use({
  renderer,
  breaks: true,
  gfm: true,
})

/**
 * Render Markdown content
 * @param content Markdown text
 * @returns Rendered HTML string
 */
export const renderMarkdown = (content?: string): string => {
  return marked.parse(content || '') as string
}
