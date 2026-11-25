export type Language = 'ru' | 'en';
export type PinnedLayout = 'keep' | 'en' | 'ru';
export type ThemeMode = 'system' | 'light' | 'dark';
export type SnippetActivation = 'space' | 'tab' | 'instant';
export type WordSource = 'manual' | 'auto_learned';

export interface LayoutConfig {
  auto_switch: boolean;
  fix_capslock: boolean;
  single_symbol_swap: boolean;
  auto_learn_enabled: boolean;
  auto_learn_threshold: number;
  switch_system_layout: boolean;
}

export interface TypoConfig {
  enabled: boolean;
  min_word_length: number;
}

export interface ListsCaseConfig {
  auto_capitalize_sentences: boolean;
  smart_lists: boolean;
}

export interface SnippetEntry {
  id: string;
  triggers: string[];
  replacement: string;
  activation: SnippetActivation;
  any_layout: boolean;
  enabled: boolean;
}

export interface UserWordEntry {
  word: string;
  source: WordSource;
}

export interface AppRule {
  process_name: string;
  display_name: string;
  disable_auto_switch: boolean;
  disable_typos: boolean;
  disable_snippets: boolean;
  disable_lists: boolean;
  disable_all: boolean;
  pinned_layout: PinnedLayout;
}

export interface HotkeyConfig {
  convert_selection_or_word: string;
  undo_last_action: string;
  swap_last_symbol: string;
  cycle_case: string;
  add_to_dictionary: string;
  screenshot_region: string;
  screenshot_window: string;
  screenshot_fullscreen: string;
}

export interface ScreenshotConfig {
  enabled: boolean;
  save_dir: string;
  filename_pattern: string;
  show_magnifier: boolean;
  copy_after_save: boolean;
  default_color: string;
  default_stroke_width: number;
}

export interface SystemConfig {
  theme: ThemeMode;
  autostart: boolean;
  show_caret_badge: boolean;
  caret_badge_duration_ms: number;
}

export interface KeyCraftConfig {
  version: number;
  enabled: boolean;
  layout: LayoutConfig;
  typos: TypoConfig;
  lists_case: ListsCaseConfig;
  snippets: SnippetEntry[];
  dictionary: UserWordEntry[];
  app_rules: AppRule[];
  hotkeys: HotkeyConfig;
  screenshot: ScreenshotConfig;
  system: SystemConfig;
}

export interface RuntimeStatus {
  enabled: boolean;
  layout: Language;
  ru_dict_size: number;
  en_dict_size: number;
  autostart: boolean;
  version: string;
  config_path: string;
  default_screenshots_dir: string;
}

export interface RunningAppInfo {
  process_name: string;
  window_title: string;
}

export interface ScreenRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CaptureMetadata {
  origin_x: number;
  origin_y: number;
  width: number;
  height: number;
  mode: string;
  active_window_rect: ScreenRect | null;
  visible_windows: ScreenRect[];
  show_magnifier: boolean;
  default_color: string;
  default_stroke_width: number;
}

export type EngineAction =
  | { type: 'no_op' }
  | {
      type: 'replace_text';
      backspace_count: number;
      text: string;
      switch_layout: Language | null;
      reason: string;
      elapsed_us: number;
      auto_learned_word: string | null;
    }
  | {
      type: 'insert_text';
      text: string;
      reason: string;
      elapsed_us: number;
    };

export type SandboxInputDto =
  | { kind: 'char'; ch: string; layout: Language }
  | { kind: 'space'; layout: Language }
  | { kind: 'tab'; layout: Language }
  | { kind: 'enter'; layout: Language }
  | { kind: 'backspace' }
  | { kind: 'reset' }
  | { kind: 'undo' }
  | { kind: 'swap_symbol'; layout: Language }
  | { kind: 'manual_convert' }
  | { kind: 'cycle_case' };

export type NavSection =
  | 'layout'
  | 'dictionary'
  | 'snippets'
  | 'lists'
  | 'apps'
  | 'screenshot'
  | 'hotkeys'
  | 'system';
