export const AGENT_COLORS = [
  '#D08040', // orange
  '#4A9A4A', // green
  '#5064A8', // indigo
  '#9B4F9B', // purple
  '#C44040', // red
  '#4090C4', // sky
  '#808040', // olive
  '#408080', // teal
]

export const getAgentColor = (index: number) => AGENT_COLORS[index % AGENT_COLORS.length]
