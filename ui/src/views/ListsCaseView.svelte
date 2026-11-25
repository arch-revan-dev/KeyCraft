<script lang="ts">
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import Toggle from '../components/Toggle.svelte';
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
  <SettingGroup title="Регистр и капитализация">
    <SettingRow
      title="Циклическая смена регистра"
      description="Последовательно переключает выделение или последнее слово: нижний → Предложение → ВЕРХНИЙ → Каждое Слово"
    >
      <KeyRecorder
        value={config.hotkeys.cycle_case}
        defaultValue="Alt+Shift+U"
        onchange={(v) => patch((c) => (c.hotkeys.cycle_case = v))}
      />
    </SettingRow>

    <SettingRow
      title="Автокапитализация начала предложения"
      description="Автоматически делает первую букву слова заглавной после точки, восклицательного или вопросительного знака"
    >
      <Toggle
        checked={config.lists_case.auto_capitalize_sentences}
        ariaLabel="Автокапитализация начала предложения"
        onchange={(v) => patch((c) => (c.lists_case.auto_capitalize_sentences = v))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Умные списки">
    <SettingRow
      title="Автопродолжение списков по Enter"
      description="Поддерживает нумерованные (1. , 1) ) и маркированные (- , • , * ) списки в любых текстовых полях"
    >
      <Toggle
        checked={config.lists_case.smart_lists}
        ariaLabel="Автопродолжение списков по Enter"
        onchange={(v) => patch((c) => (c.lists_case.smart_lists = v))}
      />
    </SettingRow>

    <div class="hint-box">
      <div class="hint-row">
        <span class="hint-label">Продолжение</span>
        <span class="hint-sample mono">1. Задача <kbd class="kbd">Enter</kbd> → 2. </span>
      </div>
      <div class="hint-row">
        <span class="hint-label">Выход из списка</span>
        <span class="hint-sample mono">Повторный <kbd class="kbd">Enter</kbd> на пустом пункте удаляет маркер</span>
      </div>
    </div>
  </SettingGroup>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .hint-box {
    border-top: 1px solid var(--border-subtle);
    padding: 12px 14px;
    background: var(--bg-content);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hint-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
  }

  .hint-label {
    color: var(--text-secondary);
  }

  .hint-sample {
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
  }
</style>
