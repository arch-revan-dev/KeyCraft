<script lang="ts">
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import Toggle from '../components/Toggle.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import KeyRecorder from '../components/KeyRecorder.svelte';
  import { cloneConfig } from '../lib/ipc';
  import type { KeyCraftConfig } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  function patch(mutator: (draft: KeyCraftConfig) => void) {
    const next = cloneConfig(config);
    mutator(next);
    onupdate(next);
  }
</script>

<div class="view-stack">
  <SettingGroup title="Автопереключение раскладки (RU ↔ EN)">
    <SettingRow
      title="Исправлять раскладку слова автоматически"
      description="Мгновенная замена при пробеле или знаке препинания: ghbdtn → привет, дуеэы → let's"
    >
      <Toggle
        checked={config.layout.auto_switch}
        ariaLabel="Исправлять раскладку слова автоматически"
        onchange={(v) => patch((c) => (c.layout.auto_switch = v))}
      />
    </SettingRow>

    <SettingRow
      title="Переключать системную раскладку после замены"
      description="Дальнейший ввод продолжится в исправленном языке без ручного переключения"
    >
      <Toggle
        checked={config.layout.switch_system_layout}
        disabled={!config.layout.auto_switch}
        ariaLabel="Переключать системную раскладку после замены"
        onchange={(v) => patch((c) => (c.layout.switch_system_layout = v))}
      />
    </SettingRow>

    <SettingRow
      title="Исправлять случайный CapsLock и две заглавные"
      description="Чинит инвертированный регистр и залипший Shift: пРИВЕТ → Привет, ЧТОбы → Чтобы"
    >
      <Toggle
        checked={config.layout.fix_capslock}
        ariaLabel="Исправлять случайный CapsLock"
        onchange={(v) => patch((c) => (c.layout.fix_capslock = v))}
      />
    </SettingRow>

    <SettingRow
      title="Постфактум-замена одиночного спецсимвола"
      description={'Меняет последний введённый символ на парный из другой раскладки: № ↔ #, " ↔ @, ; ↔ $'}
    >
      <Toggle
        checked={config.layout.single_symbol_swap}
        ariaLabel="Постфактум-замена одиночного спецсимвола"
        onchange={(v) => patch((c) => (c.layout.single_symbol_swap = v))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Офлайн-исправление опечаток">
    <SettingRow
      title="Исправлять очевидные опечатки (DL = 1)"
      description="Перестановка, пропуск, лишняя буква или промах по соседней клавише при однозначном совпадении"
    >
      <Toggle
        checked={config.typos.enabled}
        ariaLabel="Исправлять очевидные опечатки"
        onchange={(v) => patch((c) => (c.typos.enabled = v))}
      />
    </SettingRow>

    <SettingRow
      title="Минимальная длина слова"
      description="Короткие сокращения и предлоги меньше этого порога не затрагиваются"
    >
      <SegmentedControl
        options={[
          { value: '3', label: '3 буквы' },
          { value: '4', label: '4 буквы' },
          { value: '5', label: '5 букв' },
        ]}
        value={String(config.typos.min_word_length)}
        onchange={(v) => patch((c) => (c.typos.min_word_length = Number(v)))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Отмена и локальное автообучение">
    <SettingRow
      title="Отмена последнего исправления"
      description="Возвращает исходное слово и прежнюю раскладку одним нажатием"
    >
      <KeyRecorder
        value={config.hotkeys.undo_last_action}
        allowLoneModifier={true}
        onchange={(v) => patch((c) => (c.hotkeys.undo_last_action = v))}
      />
    </SettingRow>

    <SettingRow
      title="Добавлять отменённые слова в белый список"
      description="Если вы несколько раз отменили исправление одного слова, оно защищается автоматически"
    >
      <Toggle
        checked={config.layout.auto_learn_enabled}
        ariaLabel="Добавлять отменённые слова в белый список"
        onchange={(v) => patch((c) => (c.layout.auto_learn_enabled = v))}
      />
    </SettingRow>

    <SettingRow
      title="Порог отмен для запоминания"
      description="Количество отмен одного слова перед добавлением в пользовательский словарь"
    >
      <SegmentedControl
        options={[
          { value: '2', label: '2 отмены' },
          { value: '3', label: '3 отмены' },
          { value: '4', label: '4 отмены' },
        ]}
        value={String(config.layout.auto_learn_threshold)}
        onchange={(v) => patch((c) => (c.layout.auto_learn_threshold = Number(v)))}
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
