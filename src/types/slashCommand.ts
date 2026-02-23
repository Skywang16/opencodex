export interface SlashCommand {
  id: string
  icon: string
  label: string
  description?: string
  group?: string
  badge?: string
  type: 'template' | 'action'
}

export const SLASH_COMMAND_ICONS: Record<string, string> = {
  sparkles: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/></svg>`,
  mcp: `<svg viewBox="19 14 148 176" fill="none" stroke="currentColor" stroke-width="12" stroke-linecap="round"><path d="M25 97.85l67.88-67.88a24 24 0 0 1 33.94 0 24 24 0 0 1 0 33.94l-51.26 51.27"/><path d="M76.27 114.47l50.55-50.56a24 24 0 0 1 33.94 0l.35.36a24 24 0 0 1 0 33.94l-61.39 61.39a4.24 4.24 0 0 0 0 5.99l12.6 12.61"/><path d="M109.85 46.94l-50.2 50.2a24 24 0 0 0 0 33.95 24 24 0 0 0 33.94 0l50.2-50.2"/></svg>`,
  user: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="8" r="5"/><path d="M20 21a8 8 0 0 0-16 0"/></svg>`,
  plan: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1"/><path d="M9 12h6"/><path d="M9 16h6"/></svg>`,
  wand: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M15 4V2"/><path d="M15 16v-2"/><path d="M8 9h2"/><path d="M20 9h2"/><path d="M17.8 11.8 19 13"/><path d="M15 9h.01"/><path d="M17.8 6.2 19 5"/><path d="m3 21 9-9"/><path d="M12.2 6.2 11 5"/></svg>`,
  download: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`,
  skill: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/></svg>`,
}

type TranslateFn = (key: string) => string

export const createSlashCommands = (t: TranslateFn): SlashCommand[] => [
  {
    id: 'code-review',
    icon: 'sparkles',
    label: t('slash_commands.code_review'),
    type: 'template',
  },
  {
    id: 'mcp',
    icon: 'mcp',
    label: t('slash_commands.mcp'),
    description: t('slash_commands.mcp_description'),
    type: 'action',
  },
  {
    id: 'plan-mode',
    icon: 'plan',
    label: t('slash_commands.plan_mode'),
    description: t('slash_commands.plan_mode_description'),
    type: 'action',
  },
  {
    id: 'skill-creator',
    icon: 'wand',
    label: t('slash_commands.skill_creator'),
    description: t('slash_commands.skill_creator_description'),
    group: 'Skills',
    badge: t('slash_commands.badge_system'),
    type: 'template',
  },
  {
    id: 'skill-installer',
    icon: 'download',
    label: t('slash_commands.skill_installer'),
    description: t('slash_commands.skill_installer_description'),
    group: 'Skills',
    badge: t('slash_commands.badge_system'),
    type: 'action',
  },
]
