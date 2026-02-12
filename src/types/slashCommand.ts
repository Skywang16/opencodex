export interface SlashCommand {
  id: string
  icon: string
  labelKey: string
  descriptionKey?: string
  group?: string
  badge?: string
  type: 'template' | 'action'
}

export const SLASH_COMMAND_ICONS: Record<string, string> = {
  sparkles: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/></svg>`,
  server: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2" y="2" width="20" height="8" rx="2"/><rect x="2" y="14" width="20" height="8" rx="2"/><line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/></svg>`,
  user: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="8" r="5"/><path d="M20 21a8 8 0 0 0-16 0"/></svg>`,
  list: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 3v12"/><path d="m8 11 4 4 4-4"/><path d="M8 5H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-4"/></svg>`,
  wand: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M15 4V2"/><path d="M15 16v-2"/><path d="M8 9h2"/><path d="M20 9h2"/><path d="M17.8 11.8 19 13"/><path d="M15 9h.01"/><path d="M17.8 6.2 19 5"/><path d="m3 21 9-9"/><path d="M12.2 6.2 11 5"/></svg>`,
  download: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`,
}

export const SLASH_COMMANDS: SlashCommand[] = [
  {
    id: 'code-review',
    icon: 'sparkles',
    labelKey: 'slash_commands.code_review',
    type: 'template',
  },
  {
    id: 'mcp',
    icon: 'server',
    labelKey: 'slash_commands.mcp',
    descriptionKey: 'slash_commands.mcp_description',
    type: 'action',
  },
  {
    id: 'plan-mode',
    icon: 'list',
    labelKey: 'slash_commands.plan_mode',
    descriptionKey: 'slash_commands.plan_mode_description',
    type: 'action',
  },
  {
    id: 'skill-creator',
    icon: 'wand',
    labelKey: 'slash_commands.skill_creator',
    descriptionKey: 'slash_commands.skill_creator_description',
    group: 'Skills',
    badge: 'System',
    type: 'template',
  },
  {
    id: 'skill-installer',
    icon: 'download',
    labelKey: 'slash_commands.skill_installer',
    descriptionKey: 'slash_commands.skill_installer_description',
    group: 'Skills',
    badge: 'System',
    type: 'action',
  },
]
