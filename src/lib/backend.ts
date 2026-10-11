import { invoke } from '@tauri-apps/api/core';

export interface ShutdSettings {
  startTime: string;
  snoozeInterval: number;
  before: number;
  duration: number;
  autostart: boolean;
}

export interface ShutdState {
  shutdownAt: string;
  snoozed: number;
  toast: { dismissAt: string } | null;
  settings: ShutdSettings;
}

export interface ConfigInput {
  startTime: string;
  snoozeInterval: number;
  notification: { before: number; duration: number };
}

export const getState = () => invoke<ShutdState>('get_state');
export const snooze = () => invoke<ShutdState>('snooze');
export const dismissToast = () => invoke<ShutdState>('dismiss_toast');
export const setSettings = (config: ConfigInput) => invoke<ShutdState>('set_settings', { config });
export const setAutostart = (enabled: boolean) => invoke<void>('set_autostart', { enabled });
