import { themeAPI } from '@/api/config'
import { convertThemeToXTerm, type XTermTheme } from './themeConverter'

export const getTerminalThemeFromConfig = async (): Promise<XTermTheme> => {
  const theme = await themeAPI.getCurrentTheme()
  return convertThemeToXTerm(theme)
}
