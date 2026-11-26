<script lang="ts">
  import {
    Languages,
    BookOpen,
    Replace,
    ListOrdered,
    AppWindow,
    Camera,
    Keyboard,
    SlidersHorizontal,
  } from 'lucide-svelte';
  import type { NavSection } from '../lib/types';

  interface Props {
    active: NavSection;
    enabled: boolean;
    onselect: (section: NavSection) => void;
    ontoggleEnabled: () => void;
  }

  let { active, enabled, onselect, ontoggleEnabled }: Props = $props();

  const items: { id: NavSection; label: string; icon: typeof Languages }[] = [
    { id: 'layout', label: 'Раскладка и опечатки', icon: Languages },
    { id: 'dictionary', label: 'Словарь', icon: BookOpen },
    { id: 'snippets', label: 'Автоподмены', icon: Replace },
    { id: 'lists', label: 'Списки и регистр', icon: ListOrdered },
    { id: 'apps', label: 'Приложения', icon: AppWindow },
    { id: 'screenshot', label: 'Скриншотер', icon: Camera },
    { id: 'hotkeys', label: 'Горячие клавиши', icon: Keyboard },
    { id: 'system', label: 'Тест-драйв и система', icon: SlidersHorizontal },
  ];
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="brand-mark" aria-hidden="true">K</div>
    <div class="brand-text">
      <div class="brand-name">KeyCraft</div>
      <button type="button" class="status-pill" onclick={ontoggleEnabled}>
        <span class="status-dot" class:paused={!enabled}></span>
        <span>{enabled ? 'Активен' : 'На паузе'}</span>
      </button>
    </div>
  </div>

  <nav class="nav" aria-label="Разделы настроек">
    {#each items as item (item.id)}
      {@const Icon = item.icon}
      <button
        type="button"
        class="nav-item"
        class:active={active === item.id}
        onclick={() => onselect(item.id)}
      >
        <Icon size={15} strokeWidth={1.5} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="sidebar-footer">
    <span class="footer-meta">v0.1.7 · Офлайн</span>
  </div>
</aside>

<style>
  .sidebar {
    width: 210px;
    flex-shrink: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    padding: 12px 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 14px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 8px;
  }

  .brand-mark {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: #1c1c1f;
    color: #f4f4f5;
    border: 1px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
    position: relative;
    flex-shrink: 0;
  }

  .brand-mark::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 3px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .brand-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
  }

  .status-pill:hover {
    color: var(--text-primary);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--semantic-success);
  }

  .status-dot.paused {
    background: var(--text-tertiary);
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }

  .nav-item:hover {
    background: var(--bg-surface-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: var(--shadow-subtle);
  }

  .nav-item.active :global(svg) {
    color: var(--accent);
  }

  .sidebar-footer {
    padding: 10px 8px 2px;
    border-top: 1px solid var(--border-subtle);
    margin-top: 8px;
  }

  .footer-meta {
    font-size: 11px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
</style>
