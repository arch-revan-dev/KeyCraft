<script lang="ts">
  interface Props {
    value: string;
    allowLoneModifier?: boolean;
    onchange?: (next: string) => void;
  }

  let { value = $bindable(''), allowLoneModifier = false, onchange }: Props = $props();

  let recording = $state(false);
  let pendingLoneMod = $state<string | null>(null);

  const tokens = $derived(
    value
      .split('+')
      .map((s) => s.trim())
      .filter(Boolean)
  );

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

  function handleKeyDown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

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
  title="Нажмите, чтобы назначить сочетание клавиш"
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

<style>
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
</style>
