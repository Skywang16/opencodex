/**
 * System-level context menu Composable API
 *
 * Implements system-level context menu functionality using Tauri's native menu API
 */

import { ref, readonly } from 'vue'
import { Menu, MenuItem } from '@tauri-apps/api/menu'
import { LogicalPosition } from '@tauri-apps/api/dpi'

export interface PopoverMenuItem {
  label: string
  value?: unknown
  disabled?: boolean
  onClick?: () => void
}

export interface PopoverOptions {
  x: number
  y: number
  items: PopoverMenuItem[]
}

/**
 * Show system-level context menu
 */
export const showContextMenu = async (options: PopoverOptions): Promise<void> => {
  if (options.items.length === 0) return

  const menuItems = []

  for (const item of options.items) {
    menuItems.push(
      await MenuItem.new({
        id: String(item.value || item.label),
        text: item.label,
        enabled: !item.disabled,
        action: () => {
          if (!item.disabled && item.onClick) {
            item.onClick()
          }
        },
      })
    )
  }

  if (menuItems.length > 0) {
    const menu = await Menu.new({ items: menuItems })
    const position = new LogicalPosition(options.x, options.y)
    await menu.popup(position)
  }
}

/**
 * Create Popover instance
 *
 * Used to replace the original Popover component
 */
export const createPopover = () => {
  const visible = ref(false)
  const currentItems = ref<PopoverMenuItem[]>([])

  const show = async (x: number, y: number, items: PopoverMenuItem[]) => {
    currentItems.value = items
    visible.value = true

    await showContextMenu({ x, y, items })

    // Update state after menu automatically closes
    visible.value = false
  }

  const hide = () => {
    visible.value = false
    currentItems.value = []
  }

  return {
    visible: readonly(visible),
    show,
    hide,
  }
}

/**
 * Convenience function: Show menu directly at specified position
 */
export const showPopoverAt = async (x: number, y: number, items: PopoverMenuItem[]): Promise<void> => {
  await showContextMenu({ x, y, items })
}
