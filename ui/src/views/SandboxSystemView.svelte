<script lang="ts">
  import { Download, Upload, RotateCcw, Eraser } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import Toggle from '../components/Toggle.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import {
    cloneConfig,
    exportConfigJson,
    importConfigJson,
    isTauriRuntime,
    resetConfig,
    sandboxProcessEvent,
    sandboxReset,
  } from '../lib/ipc';
  import type { EngineAction, KeyCraftConfig, Language, RuntimeStatus, ThemeMode } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    status: RuntimeStatus | null;
    onupdate: (next: KeyCraftConfig) => void;
  }

  interface TraceItem {
    id: number;
    summary: string;
    latency: string;
  }

  let { config, status, onupdate }: Props = $props();

  let sandboxText = $state('');
  let sandboxLayout = $state<Language>('en');
  let trace = $state<TraceItem[]>([]);
  let importStatus = $state('');
  let fileInput: HTMLInputElement | null = null;
  let traceSeq = 0;
  let eventQueue: Promise<void> = Promise.resolve();

  function enqueueSandbox(task: () => Promise<void>) {
    eventQueue = eventQueue.then(task).catch(() => {});
  }

  function patch(mutator: (draft: KeyCraftConfig) => void) {
    const next = cloneConfig(config);
    mutator(next);
    onupdate(next);
  }

  function pushTrace(summary: string, elapsedUs: number) {
    traceSeq += 1;
    const latency = elapsedUs < 1000 ? `${elapsedUs} мкс` : `${(elapsedUs / 1000).toFixed(2)} мс`;
    trace = [{ id: traceSeq, summary, latency }, ...trace].slice(0, 6);
  }

  function applyActionToSandbox(action: EngineAction) {
    if (action.type === 'no_op') return;
    if (action.type === 'replace_text') {
      const chars = Array.from(sandboxText);
      const keep = Math.max(0, chars.length - action.backspace_count);
      sandboxText = chars.slice(0, keep).join('') + action.text;
      if (action.switch_layout) {
        sandboxLayout = action.switch_layout;
      }
      pushTrace(`${action.reason} → «${action.text.trimEnd()}»`, action.elapsed_us);
    } else if (action.type === 'insert_text') {
      sandboxText += action.text;
      pushTrace(`${action.reason}`, action.elapsed_us);
    }
  }

  // Deterministic preview helper when running Vite dev server outside Tauri
  function browserPreviewEvaluate(word: string): { text: string; reason: string; layout?: Language } | null {
    const map: Record<string, { text: string; reason: string; layout?: Language }> = {
      ghbdtn: { text: 'привет', reason: 'раскладка → RU', layout: 'ru' },
      Ghbdtn: { text: 'Привет', reason: 'раскладка → RU', layout: 'ru' },
      руддщ: { text: 'hello', reason: 'раскладка → EN', layout: 'en' },
      дуеэы: { text: "let's", reason: 'раскладка → EN', layout: 'en' },
      привте: { text: 'привет', reason: 'исправление опечатки' },
      '---': { text: '—', reason: 'сниппет (mdash)' },
      'р-': { text: '₽', reason: 'сниппет (rub)' },
      '->': { text: '→', reason: 'сниппет (arrow)' },
    };
    return map[word] ?? null;
  }

  function handleSandboxKeyDown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;

    if (!isTauriRuntime()) {
      if (e.key === ' ') {
        const parts = sandboxText.split(/\s+/);
        const last = parts[parts.length - 1] ?? '';
        const hit = browserPreviewEvaluate(last);
        if (hit) {
          e.preventDefault();
          sandboxText = sandboxText.slice(0, sandboxText.length - last.length) + hit.text + ' ';
          if (hit.layout) sandboxLayout = hit.layout;
          pushTrace(`${hit.reason} → «${hit.text}»`, 140);
        }
      }
      return;
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      enqueueSandbox(async () => {
        const act = await sandboxProcessEvent({ kind: 'undo' });
        applyActionToSandbox(act);
      });
      return;
    }

    if (e.key === 'Backspace') {
      e.preventDefault();
      const chars = Array.from(sandboxText);
      chars.pop();
      sandboxText = chars.join('');
      enqueueSandbox(async () => {
        await sandboxProcessEvent({ kind: 'backspace' });
      });
      return;
    }

    if (e.key === ' ') {
      e.preventDefault();
      const layout = sandboxLayout;
      sandboxText += ' ';
      enqueueSandbox(async () => {
        const act = await sandboxProcessEvent({ kind: 'space', layout });
        applyActionToSandbox(act);
      });
      return;
    }

    if (e.key === 'Tab') {
      e.preventDefault();
      const layout = sandboxLayout;
      sandboxText += '\t';
      enqueueSandbox(async () => {
        const act = await sandboxProcessEvent({ kind: 'tab', layout });
        applyActionToSandbox(act);
      });
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      const layout = sandboxLayout;
      sandboxText += '\n';
      enqueueSandbox(async () => {
        const act = await sandboxProcessEvent({ kind: 'enter', layout });
        applyActionToSandbox(act);
      });
      return;
    }

    if (e.key.length === 1) {
      e.preventDefault();
      const ch = e.key;
      const detected: Language = /[а-яёА-ЯЁ]/.test(ch) ? 'ru' : /[a-zA-Z]/.test(ch) ? 'en' : sandboxLayout;
      sandboxLayout = detected;
      sandboxText += ch;
      enqueueSandbox(async () => {
        const act = await sandboxProcessEvent({ kind: 'char', ch, layout: detected });
        applyActionToSandbox(act);
      });
    }
  }

  async function clearSandbox() {
    sandboxText = '';
    trace = [];
    await sandboxReset();
  }

  async function handleExportAll() {
    const json = await exportConfigJson();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'keycraft-config.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  async function handleImportAll(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    try {
      const json = await file.text();
      const loaded = await importConfigJson(json);
      onupdate(loaded);
      importStatus = 'Настройки успешно импортированы';
    } catch (err) {
      importStatus = err instanceof Error ? err.message : 'Ошибка импорта JSON';
    } finally {
      input.value = '';
    }
  }

  async function handleResetDefaults() {
    const fresh = await resetConfig();
    onupdate(fresh);
    importStatus = 'Настройки сброшены к исходным значениям';
  }
</script>

<div class="view-stack">
  <SettingGroup title="Песочница «Тест-драйв»">
    <div class="sandbox-wrap">
      <div class="sandbox-head">
        <span class="sandbox-hint">
          Попробуйте ввести <code class="mono">ghbdtn</code>, <code class="mono">дуеэы</code>,
          <code class="mono">привте</code> или <code class="mono">---</code> и нажать <kbd class="kbd">Space</kbd>
        </span>
        <div class="sandbox-tools">
          <span class="badge mono">{sandboxLayout.toUpperCase()}</span>
          <button type="button" class="btn btn-ghost" onclick={clearSandbox}>
            <Eraser size={13} strokeWidth={1.5} />
            Очистить
          </button>
        </div>
      </div>

      <textarea
        bind:value={sandboxText}
        onkeydown={handleSandboxKeyDown}
        rows={3}
        class="sandbox-input"
        placeholder="Напечатайте текст здесь, чтобы проверить раскладку, опечатки и автоподмены…"
      ></textarea>

      {#if trace.length > 0}
        <div class="trace-list">
          {#each trace as item (item.id)}
            <div class="trace-row">
              <span class="trace-summary">{item.summary}</span>
              <span class="trace-lat mono tabular">{item.latency}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </SettingGroup>

  <SettingGroup title="Оформление и индикация">
    <SettingRow title="Тема интерфейса" description="Синхронизация с темой Windows 11 или ручной выбор">
      <SegmentedControl
        options={[
          { value: 'system' as ThemeMode, label: 'Система' },
          { value: 'light' as ThemeMode, label: 'Светлая' },
          { value: 'dark' as ThemeMode, label: 'Тёмная' },
        ]}
        value={config.system.theme}
        onchange={(v) => patch((c) => (c.system.theme = v))}
      />
    </SettingRow>

    <SettingRow
      title="Бейдж раскладки у каретки"
      description="Показывает миниатюрный индикатор RU / EN рядом с курсором на 700 мс при смене языка"
    >
      <Toggle
        checked={config.system.show_caret_badge}
        ariaLabel="Бейдж раскладки у каретки"
        onchange={(v) => patch((c) => (c.system.show_caret_badge = v))}
      />
    </SettingRow>

    <SettingRow title="Запускать вместе с Windows" description="Фоновый запуск в системном трее без открытия окна настроек">
      <Toggle
        checked={config.system.autostart}
        ariaLabel="Запускать вместе с Windows"
        onchange={(v) => patch((c) => (c.system.autostart = v))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Конфигурация и офлайн-индекс">
    <SettingRow
      title="Экспорт и импорт всех настроек"
      description="Сохраняет параметры, словарь, сниппеты и правила приложений в один локальный JSON-файл"
    >
      <div class="row-btns">
        <button type="button" class="btn" onclick={handleExportAll}>
          <Download size={13} strokeWidth={1.5} />
          Экспорт JSON
        </button>
        <button type="button" class="btn" onclick={() => fileInput?.click()}>
          <Upload size={13} strokeWidth={1.5} />
          Импорт JSON
        </button>
        <button type="button" class="btn btn-danger" onclick={handleResetDefaults}>
          <RotateCcw size={13} strokeWidth={1.5} />
          Сброс
        </button>
        <input
          bind:this={fileInput}
          type="file"
          accept="application/json,.json"
          class="hidden-input"
          onchange={handleImportAll}
        />
      </div>
    </SettingRow>

    <div class="meta-strip">
      <span class="tabular">
        Словарь FST: {status?.ru_dict_size.toLocaleString('ru-RU') ?? '50 000'} RU ·
        {status?.en_dict_size.toLocaleString('ru-RU') ?? '50 000'} EN
      </span>
      <span>Сеть: отключена (0 запросов)</span>
    </div>
  </SettingGroup>

  {#if importStatus}
    <div class="status-note">{importStatus}</div>
  {/if}
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .sandbox-wrap {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .sandbox-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .sandbox-hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .sandbox-tools {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .sandbox-input {
    width: 100%;
    resize: vertical;
    min-height: 76px;
    font-size: 13.5px;
    line-height: 1.5;
    background: var(--bg-content);
  }

  .trace-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 6px;
    border-top: 1px solid var(--border-subtle);
  }

  .trace-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .trace-lat {
    color: var(--semantic-success);
  }

  .row-btns {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hidden-input {
    display: none;
  }

  .meta-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-content);
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .status-note {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 0 2px;
  }
</style>
