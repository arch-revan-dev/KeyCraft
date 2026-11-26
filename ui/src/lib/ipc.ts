import { invoke } from '@tauri-apps/api/core';
import type {
  CaptureMetadata,
  EngineAction,
  KeyCraftConfig,
  RunningAppInfo,
  RuntimeStatus,
  SandboxInputDto,
} from './types';

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function createDefaultConfig(): KeyCraftConfig {
  return {
    version: 1,
    enabled: true,
    layout: {
      auto_switch: true,
      fix_capslock: true,
      single_symbol_swap: true,
      undo_on_backspace: false,
      auto_learn_enabled: true,
      auto_learn_threshold: 3,
      switch_system_layout: true,
    },
    typos: {
      enabled: true,
      min_word_length: 3,
    },
    lists_case: {
      auto_capitalize_sentences: true,
      smart_lists: true,
    },
    snippets: [
      { id: 'rub', triggers: ['р-', 'Р-'], replacement: '₽', activation: 'space', any_layout: false, enabled: true },
      { id: 'mdash', triggers: ['---'], replacement: '—', activation: 'space', any_layout: false, enabled: true },
      { id: 'laquo', triggers: ['<<'], replacement: '«', activation: 'instant', any_layout: false, enabled: true },
      { id: 'raquo', triggers: ['>>'], replacement: '»', activation: 'instant', any_layout: false, enabled: true },
      { id: 'plusminus', triggers: ['+-'], replacement: '±', activation: 'space', any_layout: false, enabled: true },
      { id: 'arrow', triggers: ['->'], replacement: '→', activation: 'space', any_layout: false, enabled: true },
      { id: 'copyright', triggers: ['(c)', '(C)'], replacement: '©', activation: 'instant', any_layout: false, enabled: true },
    ],
    dictionary: [],
    app_rules: [
      { process_name: 'WindowsTerminal.exe', display_name: 'Windows Terminal', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
      { process_name: 'Code.exe', display_name: 'Visual Studio Code', disable_auto_switch: true, disable_typos: true, disable_snippets: false, disable_lists: true, disable_all: false, pinned_layout: 'en' },
      { process_name: 'idea64.exe', display_name: 'IntelliJ IDEA', disable_auto_switch: true, disable_typos: true, disable_snippets: false, disable_lists: true, disable_all: false, pinned_layout: 'en' },
      { process_name: 'pwsh.exe', display_name: 'PowerShell', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
      { process_name: 'cmd.exe', display_name: 'Командная строка', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
      { process_name: 'Alacritty.exe', display_name: 'Alacritty', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
      { process_name: 'wezterm-gui.exe', display_name: 'WezTerm', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
      { process_name: 'iTerm2', display_name: 'iTerm2', disable_auto_switch: true, disable_typos: true, disable_snippets: true, disable_lists: true, disable_all: true, pinned_layout: 'en' },
    ],
    hotkeys: {
      convert_selection_or_word: 'Pause',
      undo_last_action: 'Escape',
      swap_last_symbol: 'RAlt',
      cycle_case: 'Alt+Shift+U',
      add_to_dictionary: 'Ctrl+Alt+D',
      screenshot_region: 'PrintScreen',
      screenshot_window: 'Alt+PrintScreen',
      screenshot_fullscreen: 'Ctrl+PrintScreen',
    },
    screenshot: {
      enabled: true,
      save_dir: '',
      filename_pattern: 'KeyCraft_%Y-%m-%d_%H-%M-%S',
      show_magnifier: true,
      copy_after_save: true,
      default_color: '#DC2626',
      default_stroke_width: 3,
    },
    system: {
      theme: 'system',
      autostart: false,
      show_caret_badge: true,
      caret_badge_duration_ms: 700,
    },
  };
}

export function cloneConfig(cfg: KeyCraftConfig): KeyCraftConfig {
  return JSON.parse(JSON.stringify(cfg)) as KeyCraftConfig;
}

// Browser-only dev preview state (used strictly when inspecting UI in Vite dev server outside Tauri)
let browserPreviewConfig: KeyCraftConfig = createDefaultConfig();

export async function getConfig(): Promise<KeyCraftConfig> {
  if (!isTauriRuntime()) return cloneConfig(browserPreviewConfig);
  return invoke<KeyCraftConfig>('get_config');
}

export async function saveConfig(config: KeyCraftConfig): Promise<KeyCraftConfig> {
  const plain = cloneConfig(config);
  if (!isTauriRuntime()) {
    browserPreviewConfig = plain;
    return cloneConfig(browserPreviewConfig);
  }
  return invoke<KeyCraftConfig>('save_config', { config: plain });
}

export async function resetConfig(): Promise<KeyCraftConfig> {
  if (!isTauriRuntime()) {
    browserPreviewConfig = createDefaultConfig();
    return cloneConfig(browserPreviewConfig);
  }
  return invoke<KeyCraftConfig>('reset_config');
}

export async function exportConfigJson(): Promise<string> {
  if (!isTauriRuntime()) return JSON.stringify(browserPreviewConfig, null, 2);
  return invoke<string>('export_config_json');
}

export async function importConfigJson(json: string): Promise<KeyCraftConfig> {
  if (!isTauriRuntime()) {
    const parsed = JSON.parse(json) as KeyCraftConfig;
    browserPreviewConfig = parsed;
    return cloneConfig(parsed);
  }
  return invoke<KeyCraftConfig>('import_config_json', { json });
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  if (!isTauriRuntime()) {
    return {
      enabled: browserPreviewConfig.enabled,
      layout: 'ru',
      ru_dict_size: 50000,
      en_dict_size: 50000,
      autostart: browserPreviewConfig.system.autostart,
      os_snipping_prtsc_enabled: false,
      version: '0.1.6',
      config_path: 'C:\\Users\\user\\AppData\\Roaming\\KeyCraft\\config\\config.json',
      default_screenshots_dir: 'C:\\Users\\user\\Pictures\\KeyCraft',
    };
  }
  return invoke<RuntimeStatus>('get_runtime_status');
}

export async function setOsSnippingPrtScEnabled(enabled: boolean): Promise<boolean> {
  if (!isTauriRuntime()) return enabled;
  return invoke<boolean>('set_os_snipping_prtsc_enabled', { enabled });
}

export async function listRunningApps(): Promise<RunningAppInfo[]> {
  if (!isTauriRuntime()) {
    return [
      { process_name: 'chrome.exe', window_title: 'Google Chrome' },
      { process_name: 'Telegram.exe', window_title: 'Telegram' },
      { process_name: 'notepad.exe', window_title: 'Блокнот' },
    ];
  }
  return invoke<RunningAppInfo[]>('list_running_apps');
}

export async function sandboxProcessEvent(event: SandboxInputDto): Promise<EngineAction> {
  if (!isTauriRuntime()) {
    return { type: 'no_op' };
  }
  return invoke<EngineAction>('sandbox_process_event', { event });
}

export async function sandboxReset(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>('sandbox_reset');
}

export async function triggerScreenshot(mode: 'region' | 'window' | 'fullscreen'): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>('trigger_screenshot', { mode });
}

export async function getCaptureMetadata(): Promise<CaptureMetadata> {
  if (!isTauriRuntime()) {
    return {
      origin_x: 0,
      origin_y: 0,
      width: window.innerWidth,
      height: window.innerHeight,
      mode: 'region',
      active_window_rect: { x: 120, y: 80, width: 640, height: 420 },
      visible_windows: [{ x: 120, y: 80, width: 640, height: 420 }],
      show_magnifier: true,
      default_color: '#DC2626',
      default_stroke_width: 3,
    };
  }
  return invoke<CaptureMetadata>('get_capture_metadata');
}

export async function getCaptureImageDataUrl(): Promise<string> {
  if (!isTauriRuntime()) return '';
  return invoke<string>('get_capture_image_data_url');
}

export async function screenshotCopyPng(pngBase64: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>('screenshot_copy_png', { pngBase64 });
}

export async function screenshotSavePng(pngBase64: string): Promise<string> {
  if (!isTauriRuntime()) return 'KeyCraft_preview.png';
  return invoke<string>('screenshot_save_png', { pngBase64 });
}

export async function screenshotPinPng(pngBase64: string, screenX: number, screenY: number): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>('screenshot_pin_png', { pngBase64, screenX, screenY });
}

export async function screenshotClose(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>('screenshot_close');
}
