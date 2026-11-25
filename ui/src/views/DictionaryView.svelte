<script lang="ts">
  import { BookOpen, Plus, Search, Trash2, Download, Upload } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import { cloneConfig } from '../lib/ipc';
  import type { KeyCraftConfig, UserWordEntry } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, onupdate }: Props = $props();

  let newWord = $state('');
  let query = $state('');
  let filter = $state<'all' | 'manual' | 'auto_learned'>('all');
  let statusMessage = $state('');
  let fileInput: HTMLInputElement | null = null;

  const filteredWords = $derived(
    config.dictionary.filter((entry) => {
      if (filter !== 'all' && entry.source !== filter) return false;
      if (!query.trim()) return true;
      return entry.word.toLowerCase().includes(query.trim().toLowerCase());
    })
  );

  function addWord() {
    const clean = newWord.trim().toLowerCase();
    if (!clean) return;
    if (config.dictionary.some((e) => e.word.toLowerCase() === clean)) {
      statusMessage = `Слово «${clean}» уже есть в словаре`;
      return;
    }
    const next = cloneConfig(config);
    next.dictionary.unshift({ word: clean, source: 'manual' });
    onupdate(next);
    newWord = '';
    statusMessage = '';
  }

  function removeWord(word: string) {
    const next = cloneConfig(config);
    next.dictionary = next.dictionary.filter((e) => e.word !== word);
    onupdate(next);
  }

  function exportDict() {
    const blob = new Blob([JSON.stringify(config.dictionary, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'keycraft-dictionary.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  async function importDict(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const parsed = JSON.parse(text) as unknown;
      if (!Array.isArray(parsed)) {
        statusMessage = 'Ожидается JSON-массив слов';
        return;
      }
      const next = cloneConfig(config);
      const existing = new Set(next.dictionary.map((d) => d.word.toLowerCase()));
      let added = 0;
      for (const item of parsed) {
        const w =
          typeof item === 'string'
            ? item.trim().toLowerCase()
            : typeof (item as UserWordEntry)?.word === 'string'
              ? (item as UserWordEntry).word.trim().toLowerCase()
              : '';
        if (w && !existing.has(w)) {
          existing.add(w);
          next.dictionary.push({
            word: w,
            source: (item as UserWordEntry)?.source === 'auto_learned' ? 'auto_learned' : 'manual',
          });
          added++;
        }
      }
      onupdate(next);
      statusMessage = `Импортировано слов: ${added}`;
    } catch {
      statusMessage = 'Не удалось прочитать JSON-файл словаря';
    } finally {
      input.value = '';
    }
  }
</script>

<div class="view-stack">
  <div class="toolbar">
    <form
      class="add-form"
      onsubmit={(e) => {
        e.preventDefault();
        addWord();
      }}
    >
      <input
        type="text"
        bind:value={newWord}
        placeholder="Добавить термин, англицизм или аббревиатуру…"
        class="add-input"
      />
      <button type="submit" class="btn btn-primary">
        <Plus size={14} strokeWidth={1.5} />
        Добавить
      </button>
    </form>

    <div class="toolbar-actions">
      <button type="button" class="btn" onclick={exportDict} title="Экспорт словаря в JSON">
        <Download size={13} strokeWidth={1.5} />
        Экспорт
      </button>
      <button type="button" class="btn" onclick={() => fileInput?.click()} title="Импорт словаря из JSON">
        <Upload size={13} strokeWidth={1.5} />
        Импорт
      </button>
      <input
        bind:this={fileInput}
        type="file"
        accept="application/json,.json"
        class="hidden-input"
        onchange={importDict}
      />
    </div>
  </div>

  {#if statusMessage}
    <div class="inline-note">{statusMessage}</div>
  {/if}

  <div class="filter-bar">
    <div class="search-wrap">
      <Search size={14} strokeWidth={1.5} />
      <input type="search" bind:value={query} placeholder="Поиск по словарю…" class="search-input" />
    </div>
    <SegmentedControl
      options={[
        { value: 'all', label: `Все (${config.dictionary.length})` },
        { value: 'manual', label: 'Вручную' },
        { value: 'auto_learned', label: 'Автообучение' },
      ]}
      bind:value={filter}
    />
  </div>

  <SettingGroup>
    {#if filteredWords.length === 0}
      <EmptyState
        title={config.dictionary.length === 0 ? 'Пользовательский словарь пуст' : 'Ничего не найдено'}
        description={config.dictionary.length === 0
          ? 'Добавьте профессиональные термины, бренды или сленг — KeyCraft защитит их от автопереключения и будет учитывать при конвертации.'
          : 'Измените поисковый запрос или переключите фильтр.'}
      >
        {#snippet icon()}
          <BookOpen size={16} strokeWidth={1.5} />
        {/snippet}
        {#snippet action()}
          {#if config.dictionary.length === 0}
            <button
              type="button"
              class="btn"
              onclick={() => {
                newWord = 'KeyCraft';
                addWord();
              }}
            >
              Добавить пример («keycraft»)
            </button>
          {/if}
        {/snippet}
      </EmptyState>
    {:else}
      <div class="word-list">
        {#each filteredWords as entry (entry.word)}
          <div class="word-row">
            <div class="word-left">
              <span class="word-text mono">{entry.word}</span>
              <span class="badge" class:badge-accent={entry.source === 'auto_learned'}>
                {entry.source === 'auto_learned' ? 'Автообучение' : 'Вручную'}
              </span>
            </div>
            <button
              type="button"
              class="btn btn-danger btn-icon"
              aria-label={`Удалить ${entry.word}`}
              onclick={() => removeWord(entry.word)}
            >
              <Trash2 size={14} strokeWidth={1.5} />
            </button>
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
    justify-content: space-between;
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

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hidden-input {
    display: none;
  }

  .inline-note {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 0 2px;
  }

  .filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    max-width: 280px;
    color: var(--text-tertiary);
  }

  .search-wrap :global(svg) {
    position: absolute;
    left: 9px;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    height: 28px;
    padding-left: 28px;
    font-size: 12px;
  }

  .word-list {
    display: flex;
    flex-direction: column;
  }

  .word-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px;
    min-height: 40px;
  }

  .word-row + .word-row {
    border-top: 1px solid var(--border-subtle);
  }

  .word-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .word-text {
    font-size: 12.5px;
    color: var(--text-primary);
  }
</style>
