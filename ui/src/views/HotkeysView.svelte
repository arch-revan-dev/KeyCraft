<script lang="ts">
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import KeyRecorder from '../components/KeyRecorder.svelte';
  import { cloneConfig } from '../lib/ipc';
  import type { KeyCraftConfig } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  function setHotkey(key: keyof KeyCraftConfig['hotkeys'], val: string) {
    const next = cloneConfig(config);
    next.hotkeys[key] = val;
    onupdate(next);
  }
</script>

<div class="view-stack">
  <SettingGroup title="Раскладка и текст">
    <SettingRow
      title="Конвертировать последнее слово или выделение"
      description="Принудительно переводит текст в противоположную раскладку"
    >
      <KeyRecorder
        value={config.hotkeys.convert_selection_or_word}
        onchange={(v) => setHotkey('convert_selection_or_word', v)}
      />
    </SettingRow>

    <SettingRow
      title="Отменить последнее автопереключение"
      description="Поддерживается одиночное нажатие Left Alt или клавиша Esc"
    >
      <KeyRecorder
        value={config.hotkeys.undo_last_action}
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
        onchange={(v) => setHotkey('cycle_case', v)}
      />
    </SettingRow>

    <SettingRow
      title="Добавить выделенное слово в словарь"
      description="Защищает выделенный термин от автопереключения в любом приложении"
    >
      <KeyRecorder
        value={config.hotkeys.add_to_dictionary}
        onchange={(v) => setHotkey('add_to_dictionary', v)}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Скриншотер">
    <SettingRow title="Скриншот области" description="Открывает оверлей выделения и аннотаций">
      <KeyRecorder
        value={config.hotkeys.screenshot_region}
        onchange={(v) => setHotkey('screenshot_region', v)}
      />
    </SettingRow>
    <SettingRow title="Скриншот активного окна" description="Мгновенный захват текущего окна">
      <KeyRecorder
        value={config.hotkeys.screenshot_window}
        onchange={(v) => setHotkey('screenshot_window', v)}
      />
    </SettingRow>
    <SettingRow title="Скриншот всего экрана" description="Захват всех подключённых мониторов">
      <KeyRecorder
        value={config.hotkeys.screenshot_fullscreen}
        onchange={(v) => setHotkey('screenshot_fullscreen', v)}
      />
    </SettingRow>
  </SettingGroup>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
</style>
