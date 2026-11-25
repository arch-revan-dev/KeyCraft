<script lang="ts">
  import { AppWindow, Plus, RefreshCw, Trash2 } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { cloneConfig, listRunningApps } from '../lib/ipc';
  import type { AppRule, KeyCraftConfig, PinnedLayout, RunningAppInfo } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  let manualProc = $state('');
  let runningApps = $state<RunningAppInfo[]>([]);
  let showPicker = $state(false);

  async function loadRunning() {
    runningApps = await listRunningApps();
    showPicker = true;
  }

  function addRule(processName: string, displayName?: string) {
    const clean = processName.trim();
    if (!clean) return;
    if (config.app_rules.some((r) => r.process_name.toLowerCase() === clean.toLowerCase())) {
      return;
    }
    const next = cloneConfig(config);
    next.app_rules.unshift({
      process_name: clean,
      display_name: displayName?.trim() || clean.replace(/\.exe$/i, ''),
      disable_auto_switch: true,
      disable_typos: true,
      disable_snippets: false,
      disable_lists: false,
      disable_all: false,
      pinned_layout: 'keep',
    });
    onupdate(next);
    manualProc = '';
    showPicker = false;
  }

  function patchRule(processName: string, mutator: (r: AppRule) => void) {
    const next = cloneConfig(config);
    const rule = next.app_rules.find((r) => r.process_name === processName);
    if (!rule) return;
    mutator(rule);
    onupdate(next);
  }

  function removeRule(processName: string) {
    const next = cloneConfig(config);
    next.app_rules = next.app_rules.filter((r) => r.process_name !== processName);
    onupdate(next);
  }
</script>

<div class="view-stack">
  <div class="toolbar">
    <form
      class="add-form"
      onsubmit={(e) => {
        e.preventDefault();
        addRule(manualProc);
      }}
    >
      <input
        type="text"
        bind:value={manualProc}
        placeholder="Имя процесса, например: Telegram.exe"
        class="add-input mono"
      />
      <button type="submit" class="btn btn-primary" disabled={!manualProc.trim()}>
        <Plus size={14} strokeWidth={1.5} />
        Добавить
      </button>
    </form>

    <button type="button" class="btn" onclick={loadRunning}>
      <RefreshCw size={13} strokeWidth={1.5} />
      Из открытых окон
    </button>
  </div>

  {#if showPicker}
    <SettingGroup title="Открытые приложения">
      <div class="picker-list">
        {#if runningApps.length === 0}
          <div class="picker-empty">Активных окон для добавления не обнаружено.</div>
        {:else}
          {#each runningApps as app (app.process_name)}
            <div class="picker-row">
              <div class="picker-info">
                <span class="picker-proc mono">{app.process_name}</span>
                <span class="picker-title">{app.window_title}</span>
              </div>
              <button
                type="button"
                class="btn"
                onclick={() => addRule(app.process_name, app.window_title)}
              >
                Выбрать
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </SettingGroup>
  {/if}

  <SettingGroup title={`Правила и стартовая раскладка (${config.app_rules.length})`}>
    {#if config.app_rules.length === 0}
      <EmptyState
        title="Правила для приложений не заданы"
        description="Добавьте терминал, IDE или мессенджер, чтобы отключить автоподмены или фиксировать стартовую раскладку."
      >
        {#snippet icon()}
          <AppWindow size={16} strokeWidth={1.5} />
        {/snippet}
      </EmptyState>
    {:else}
      <div class="rules-list">
        {#each config.app_rules as rule (rule.process_name)}
          <div class="rule-card">
            <div class="rule-head">
              <div class="rule-name">
                <span class="rule-display">{rule.display_name}</span>
                <span class="rule-proc mono">{rule.process_name}</span>
              </div>
              <div class="rule-right">
                <SegmentedControl
                  options={[
                    { value: 'keep' as PinnedLayout, label: 'Как есть' },
                    { value: 'en' as PinnedLayout, label: 'EN' },
                    { value: 'ru' as PinnedLayout, label: 'RU' },
                  ]}
                  value={rule.pinned_layout}
                  onchange={(v) => patchRule(rule.process_name, (r) => (r.pinned_layout = v))}
                />
                <button
                  type="button"
                  class="btn btn-danger btn-icon"
                  aria-label={`Удалить правило ${rule.process_name}`}
                  onclick={() => removeRule(rule.process_name)}
                >
                  <Trash2 size={14} strokeWidth={1.5} />
                </button>
              </div>
            </div>

            <div class="flags-row">
              <button
                type="button"
                class="flag-pill"
                class:active={rule.disable_all}
                onclick={() =>
                  patchRule(rule.process_name, (r) => {
                    r.disable_all = !r.disable_all;
                    if (r.disable_all) {
                      r.disable_auto_switch = true;
                      r.disable_typos = true;
                      r.disable_snippets = true;
                      r.disable_lists = true;
                    }
                  })}
              >
                Отключить всё
              </button>
              <button
                type="button"
                class="flag-pill"
                class:active={rule.disable_auto_switch}
                disabled={rule.disable_all}
                onclick={() =>
                  patchRule(rule.process_name, (r) => (r.disable_auto_switch = !r.disable_auto_switch))}
              >
                Без автосмены
              </button>
              <button
                type="button"
                class="flag-pill"
                class:active={rule.disable_typos}
                disabled={rule.disable_all}
                onclick={() =>
                  patchRule(rule.process_name, (r) => (r.disable_typos = !r.disable_typos))}
              >
                Без опечаток
              </button>
              <button
                type="button"
                class="flag-pill"
                class:active={rule.disable_snippets}
                disabled={rule.disable_all}
                onclick={() =>
                  patchRule(rule.process_name, (r) => (r.disable_snippets = !r.disable_snippets))}
              >
                Без сниппетов
              </button>
              <button
                type="button"
                class="flag-pill"
                class:active={rule.disable_lists}
                disabled={rule.disable_all}
                onclick={() =>
                  patchRule(rule.process_name, (r) => (r.disable_lists = !r.disable_lists))}
              >
                Без списков
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </SettingGroup>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .add-form {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .add-input {
    flex: 1;
    height: 30px;
  }

  .picker-list {
    display: flex;
    flex-direction: column;
    max-height: 160px;
    overflow-y: auto;
  }

  .picker-empty {
    padding: 14px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .picker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px;
    gap: 12px;
  }

  .picker-row + .picker-row {
    border-top: 1px solid var(--border-subtle);
  }

  .picker-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .picker-proc {
    font-size: 12px;
    color: var(--text-primary);
  }

  .picker-title {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
  }

  .rule-card {
    padding: 11px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .rule-card + .rule-card {
    border-top: 1px solid var(--border-subtle);
  }

  .rule-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .rule-name {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .rule-display {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .rule-proc {
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .rule-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .flags-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }

  .flag-pill {
    height: 24px;
    padding: 0 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-content);
    color: var(--text-secondary);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }

  .flag-pill:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .flag-pill.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .flag-pill:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    background: transparent;
    border-color: var(--border-subtle);
    color: var(--text-tertiary);
  }
</style>
