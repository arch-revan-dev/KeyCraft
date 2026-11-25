<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';

  interface Props {
    value: string;
    defaultValue?: string;
    allowLoneModifier?: boolean;
    onchange?: (next: string) => void;
  }

  let {
    value = $bindable(''),
    defaultValue,
    allowLoneModifier = false,
    onchange,
  }: Props = $props();

  let recording = $state(false);
  let pendingLoneMod = $state<string | null>(null);

  const tokens = $derived(
    value
      .split('+')
      .map((s) => s.trim())
      .filter(Boolean)
  );

  const isModified = $derived(defaultValue !== undefined && value !== defaultValue);

  function formatToken(token: string): string {
    switch (token) {
      case 'Escape':
        return 'Esc';
      case 'PrintScreen':
        return 'PrtSc';
      case 'RAlt':
        return 'Right Alt';
      case 'LAlt':
        return 'Left Alt';
      case 'Control':
        return 'Ctrl';
      default:
        return token;
    }
  }

  function startRecording() {
    recording = true;
    pendingLoneMod = null;
  }

  function finish(next: string) {
    recording = false;
    pendingLoneMod = null;
    value = next;
    onchange?.(next);
  }

  function resetToDefault() {
    if (defaultValue === undefined) return;
    finish(defaultValue);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    if ((e.key === 'Backspace' || e.key === 'Delete') && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      finish('');
      return;
    }

    if (allowLoneModifier && (e.code === 'AltRight' || e.code === 'AltLeft')) {
      pendingLoneMod = e.code === 'AltRight' ? 'RAlt' : 'LAlt';
      return;
    }

    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
      pendingLoneMod = null;
      return;
    }

    pendingLoneMod = null;
    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    if (e.metaKey) parts.push('Win');

    let keyName = e.key;
    if (e.code === 'Space') keyName = 'Space';
    else if (e.code === 'Pause') keyName = 'Pause';
    else if (e.code === 'PrintScreen') keyName = 'PrintScreen';
    else if (e.key === 'Escape') keyName = 'Escape';
    else if (keyName.length === 1) keyName = keyName.toUpperCase();

    parts.push(keyName);
    finish(parts.join('+'));
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (!recording || !pendingLoneMod) return;
    if (e.code === 'AltRight' || e.code === 'AltLeft') {
      e.preventDefault();
      finish(pendingLoneMod);
    }
  }
</script>

<div class="recorder-wrap">
  <button
    type="button"
    class="recorder"
    class:recording
    onclick={startRecording}
    onkeydown={handleKeyDown}
    onkeyup={handleKeyUp}
    onblur={() => {
      recording = false;
      pendingLoneMod = null;
    }}
    title="Нажмите, чтобы назначить сочетание (Backspace — очистить)"
  >
    {#if recording}
      <span class="rec-prompt">Нажмите сочетание…</span>
    {:else if tokens.length === 0}
      <span class="rec-empty">Не назначено</span>
    {:else}
      <span class="kbd-group">
        {#each tokens as tok, i}
          {#if i > 0}
            <span class="plus">+</span>
          {/if}
          <kbd class="kbd">{formatToken(tok)}</kbd>
        {/each}
      </span>
    {/if}
  </button>

  {#if isModified}
    <button
      type="button"
      class="reset-btn"
      title={`Сбросить по умолчанию (${defaultValue})`}
      aria-label="Сбросить горячую клавишу по умолчанию"
      onclick={resetToDefault}
    >
      <RotateCcw size={13} strokeWidth={1.5} />
    </button>
  {/if}
</div>

<style>
  .recorder-wrap {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .recorder {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 146px;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-content);
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease),
      background-color var(--dur-fast) var(--ease),
      box-shadow var(--dur-fast) var(--ease);
  }

  .recorder:hover {
    border-color: var(--border-strong);
    background: var(--bg-surface-hover);
  }

  .recorder.recording {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }

  .rec-prompt {
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
  }

  .rec-empty {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .kbd-group {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .plus {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .reset-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }

  .reset-btn:hover {
    background: var(--bg-surface-hover);
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
</style>
