<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import KeyRecorder from '../components/KeyRecorder.svelte';
  import { cloneConfig, createDefaultConfig } from '../lib/ipc';
  import type { KeyCraftConfig } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  const DEFAULTS = createDefaultConfig().hotkeys;

  const hasCustomHotkeys = $derived(
    (Object.keys(DEFAULTS) as (keyof KeyCraftConfig['hotkeys'])[]).some(
      (k) => config.hotkeys[k] !== DEFAULTS[k]
    )
  );

  function setHotkey(key: keyof KeyCraftConfig['hotkeys'], val: string) {
    const next = cloneConfig(config);
    next.hotkeys[key] = val;
    onupdate(next);
  }

  function resetAllHotkeys() {
    const next = cloneConfig(config);
    next.hotkeys = { ...DEFAULTS };
    onupdate(next);
  }
</script>

<div class="view-stack">
  <div class="top-bar">
    <span class="top-hint">Нажмите на клавишу для переназначения; <kbd class="kbd">Backspace</kbd> очищает сочетание</span>
    <button type="button" class="btn" disabled={!hasCustomHotkeys} onclick={resetAllHotkeys}>
      <RotateCcw size={13} strokeWidth={1.5} />
      Сбросить клавиши
    </button>
  </div>

  <SettingGroup title="Раскладка и текст">
    <SettingRow
      title="Конвертировать последнее слово или выделение"
      description="Принудительно переводит текст в противоположную раскладку"
    >
      <KeyRecorder
        value={config.hotkeys.convert_selection_or_word}
        defaultValue={DEFAULTS.convert_selection_or_word}
        onchange={(v) => setHotkey('convert_selection_or_word', v)}
      />
    </SettingRow>

    <SettingRow
      title="Отменить последнее автопереключение"
      description="Поддерживается одиночное нажатие Left Alt или клавиша Esc"
    >
      <KeyRecorder
        value={config.hotkeys.undo_last_action}
        defaultValue={DEFAULTS.undo_last_action}
        allowLoneModifier={true}
        onchange={(v) => setHotkey('undo_last_action', v)}
      />
    </SettingRow>

    <SettingRow
      title="Заменить последний спецсимвол (№ ↔ #)"
      description="Рекомендуется одиночное нажатие Right Alt"
    >
      <KeyRecorder
        value={config.hotkeys.swap_last_symbol}
        defaultValue={DEFAULTS.swap_last_symbol}
        allowLoneModifier={true}
        onchange={(v) => setHotkey('swap_last_symbol', v)}
      />
    </SettingRow>

    <SettingRow
      title="Циклическая смена регистра"
      description="Переключает регистр выделенного фрагмента или последнего слова"
    >
      <KeyRecorder
        value={config.hotkeys.cycle_case}
        defaultValue={DEFAULTS.cycle_case}
        onchange={(v) => setHotkey('cycle_case', v)}
      />
    </SettingRow>

    <SettingRow
      title="Добавить выделенное слово в словарь"
      description="Защищает выделенный термин от автопереключения в любом приложении"
    >
      <KeyRecorder
        value={config.hotkeys.add_to_dictionary}
        defaultValue={DEFAULTS.add_to_dictionary}
        onchange={(v) => setHotkey('add_to_dictionary', v)}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Скриншотер">
    <SettingRow title="Скриншот области" description="Открывает оверлей выделения и аннотаций">
      <KeyRecorder
        value={config.hotkeys.screenshot_region}
        defaultValue={DEFAULTS.screenshot_region}
        onchange={(v) => setHotkey('screenshot_region', v)}
      />
    </SettingRow>
    <SettingRow title="Скриншот активного окна" description="Мгновенный захват текущего окна">
      <KeyRecorder
        value={config.hotkeys.screenshot_window}
        defaultValue={DEFAULTS.screenshot_window}
        onchange={(v) => setHotkey('screenshot_window', v)}
      />
    </SettingRow>
    <SettingRow title="Скриншот всего экрана" description="Захват всех подключённых мониторов">
      <KeyRecorder
        value={config.hotkeys.screenshot_fullscreen}
        defaultValue={DEFAULTS.screenshot_fullscreen}
        onchange={(v) => setHotkey('screenshot_fullscreen', v)}
      />
    </SettingRow>
  </SettingGroup>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .top-hint {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
