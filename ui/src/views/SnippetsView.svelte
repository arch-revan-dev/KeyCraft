<script lang="ts">
  import { Plus, Trash2, Replace, Pencil, Check, X } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import Toggle from '../components/Toggle.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { cloneConfig } from '../lib/ipc';
  import type { KeyCraftConfig, SnippetActivation, SnippetEntry } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  let triggersRaw = $state('');
  let replacement = $state('');
  let activation = $state<SnippetActivation>('space');
  let anyLayout = $state(true);

  let editingId = $state<string | null>(null);
  let editTriggersRaw = $state('');
  let editReplacement = $state('');
  let editActivation = $state<SnippetActivation>('space');
  let editAnyLayout = $state(true);

  function parseTriggers(raw: string): string[] {
    return raw
      .split(/\s+/)
      .map((s) => s.trim())
      .filter(Boolean);
  }

  function addSnippet() {
    const triggers = parseTriggers(triggersRaw);
    if (triggers.length === 0 || !replacement) return;

    const next = cloneConfig(config);
    next.snippets.push({
      id: `custom-${Date.now()}`,
      triggers,
      replacement,
      activation,
      any_layout: anyLayout,
      enabled: true,
    });
    onupdate(next);
    triggersRaw = '';
    replacement = '';
    anyLayout = true;
  }

  function startEdit(snip: SnippetEntry) {
    editingId = snip.id;
    editTriggersRaw = snip.triggers.join(' ');
    editReplacement = snip.replacement;
    editActivation = snip.activation;
    editAnyLayout = snip.any_layout;
  }

  function cancelEdit() {
    editingId = null;
  }

  function saveEdit() {
    if (!editingId) return;
    const triggers = parseTriggers(editTriggersRaw);
    if (triggers.length === 0 || !editReplacement) return;
    const id = editingId;
    updateSnippet(id, (s) => {
      s.triggers = triggers;
      s.replacement = editReplacement;
      s.activation = editActivation;
      s.any_layout = editAnyLayout;
    });
    editingId = null;
  }

  function updateSnippet(id: string, mutator: (s: SnippetEntry) => void) {
    const next = cloneConfig(config);
    const target = next.snippets.find((s) => s.id === id);
    if (!target) return;
    mutator(target);
    onupdate(next);
  }

  function removeSnippet(id: string) {
    if (editingId === id) editingId = null;
    const next = cloneConfig(config);
    next.snippets = next.snippets.filter((s) => s.id !== id);
    onupdate(next);
  }

  function activationLabel(act: SnippetActivation): string {
    switch (act) {
      case 'space':
        return 'Space';
      case 'tab':
        return 'Tab';
      case 'instant':
        return 'Мгновенно';
    }
  }
</script>

<div class="view-stack">
  <SettingGroup title="Новая автоподмена">
    <form
      class="new-form"
      onsubmit={(e) => {
        e.preventDefault();
        addSnippet();
      }}
    >
      <div class="form-grid">
        <label class="field">
          <span class="field-label">Триггеры (через пробел)</span>
          <input
            type="text"
            bind:value={triggersRaw}
            placeholder="+админ +mail"
            class="mono"
          />
        </label>
        <label class="field">
          <span class="field-label">Замена</span>
          <input type="text" bind:value={replacement} placeholder="admin@company.local" />
        </label>
      </div>

      <div class="form-footer">
        <div class="form-opts">
          <SegmentedControl
            options={[
              { value: 'space', label: 'По Space' },
              { value: 'tab', label: 'По Tab' },
              { value: 'instant', label: 'Мгновенно' },
            ]}
            bind:value={activation}
          />
          <label class="check-label">
            <Toggle bind:checked={anyLayout} ariaLabel="Срабатывать в любой раскладке" />
            <span>В любой раскладке</span>
          </label>
        </div>

        <button type="submit" class="btn btn-primary" disabled={!triggersRaw.trim() || !replacement}>
          <Plus size={14} strokeWidth={1.5} />
          Добавить
        </button>
      </div>
    </form>
  </SettingGroup>

  <SettingGroup title={`Активные подмены (${config.snippets.length})`}>
    {#if config.snippets.length === 0}
      <EmptyState
        title="Список автоподмен пуст"
        description="Создайте короткие триггеры для типографских знаков, email-адресов или часто повторяемых фраз."
      >
        {#snippet icon()}
          <Replace size={16} strokeWidth={1.5} />
        {/snippet}
      </EmptyState>
    {:else}
      <div class="snippet-table">
        {#each config.snippets as snip (snip.id)}
          {#if editingId === snip.id}
            <form
              class="snippet-edit-form"
              onsubmit={(e) => {
                e.preventDefault();
                saveEdit();
              }}
              onkeydown={(e) => {
                if (e.key === 'Escape') cancelEdit();
              }}
            >
              <div class="form-grid">
                <label class="field">
                  <span class="field-label">Триггеры (через пробел)</span>
                  <input type="text" bind:value={editTriggersRaw} class="mono" />
                </label>
                <label class="field">
                  <span class="field-label">Замена</span>
                  <input type="text" bind:value={editReplacement} />
                </label>
              </div>
              <div class="form-footer">
                <div class="form-opts">
                  <SegmentedControl
                    options={[
                      { value: 'space', label: 'По Space' },
                      { value: 'tab', label: 'По Tab' },
                      { value: 'instant', label: 'Мгновенно' },
                    ]}
                    bind:value={editActivation}
                  />
                  <label class="check-label">
                    <Toggle bind:checked={editAnyLayout} ariaLabel="Срабатывать в любой раскладке" />
                    <span>В любой раскладке</span>
                  </label>
                </div>
                <div class="edit-actions">
                  <button type="button" class="btn btn-ghost" onclick={cancelEdit}>
                    <X size={14} strokeWidth={1.5} />
                    Отмена
                  </button>
                  <button
                    type="submit"
                    class="btn btn-primary"
                    disabled={!editTriggersRaw.trim() || !editReplacement}
                  >
                    <Check size={14} strokeWidth={1.5} />
                    Сохранить
                  </button>
                </div>
              </div>
            </form>
          {:else}
            <div class="snippet-row" class:disabled={!snip.enabled}>
              <div class="snip-main">
                <div class="snip-triggers">
                  {#each snip.triggers as trig}
                    <code class="trig-chip mono">{trig}</code>
                  {/each}
                  <span class="arrow">→</span>
                  <span class="snip-repl">{snip.replacement}</span>
                </div>
                <div class="snip-meta">
                  <span class="badge">{activationLabel(snip.activation)}</span>
                  {#if snip.any_layout}
                    <span class="badge badge-accent">Любая раскладка</span>
                  {/if}
                </div>
              </div>

              <div class="snip-actions">
                <Toggle
                  checked={snip.enabled}
                  ariaLabel={`Включить сниппет ${snip.triggers.join(' ')}`}
                  onchange={(v) => updateSnippet(snip.id, (s) => (s.enabled = v))}
                />
                <button
                  type="button"
                  class="btn btn-ghost btn-icon"
                  title="Редактировать автоподмену"
                  aria-label="Редактировать автоподмену"
                  onclick={() => startEdit(snip)}
                >
                  <Pencil size={14} strokeWidth={1.5} />
                </button>
                <button
                  type="button"
                  class="btn btn-danger btn-icon"
                  title="Удалить автоподмену"
                  aria-label="Удалить автоподмену"
                  onclick={() => removeSnippet(snip.id)}
                >
                  <Trash2 size={14} strokeWidth={1.5} />
                </button>
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </SettingGroup>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .new-form {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-label {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .form-opts {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .check-label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .snippet-table {
    display: flex;
    flex-direction: column;
  }

  .snippet-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    min-height: 46px;
  }

  .snippet-row + .snippet-row,
  .snippet-row + .snippet-edit-form,
  .snippet-edit-form + .snippet-row {
    border-top: 1px solid var(--border-subtle);
  }

  .snippet-edit-form {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg-control);
  }

  .edit-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .snippet-row.disabled {
    opacity: 0.55;
  }

  .snip-main {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .snip-triggers {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }

  .trig-chip {
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--bg-control);
    border: 1px solid var(--border-subtle);
    font-size: 12px;
    color: var(--text-primary);
  }

  .arrow {
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .snip-repl {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    word-break: break-word;
  }

  .snip-meta {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .snip-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
</style>
