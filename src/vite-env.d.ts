/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}

// Extend Window interface for development environment global functions
declare global {
  interface Window {
    reloadShortcuts?: (() => void);
  }
}
