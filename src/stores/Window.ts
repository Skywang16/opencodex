import { defineStore } from 'pinia'
import { windowApi } from '@/api/window'

export const useWindowStore = defineStore('window', {
  state: () => ({
    alwaysOnTop: false as boolean,
  }),
  actions: {
    setAlwaysOnTop: function (value: boolean) {
      this.alwaysOnTop = value
    },
    initFromSystem: async function () {
      try {
        const state = await windowApi.getState(false)
        this.alwaysOnTop = !!state.alwaysOnTop
      } catch (e) {
        console.warn('Failed to initialize window state from system, keeping defaults:', e)
      }
    },
  },
})
