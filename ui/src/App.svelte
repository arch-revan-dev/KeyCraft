<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import LayoutTyposView from './views/LayoutTyposView.svelte';
  import DictionaryView from './views/DictionaryView.svelte';
  import SnippetsView from './views/SnippetsView.svelte';
  import ListsCaseView from './views/ListsCaseView.svelte';
  import AppRulesView from './views/AppRulesView.svelte';
  import ScreenshotView from './views/ScreenshotView.svelte';
  import HotkeysView from './views/HotkeysView.svelte';
  import SandboxSystemView from './views/SandboxSystemView.svelte';
  import {
    cloneConfig,
    createDefaultConfig,
    getConfig,
    getRuntimeStatus,
    saveConfig,
  } from './lib/ipc';
  import { applyTheme, initThemeListener } from './lib/theme';
  import type { KeyCraftConfig, NavSection, RuntimeStatus } from './lib/types';

  let activeSection = $state<NavSection>('layout');
  let config = $state<KeyCraftConfig>(createDefaultConfig());
  let status = $state<RuntimeStatus | null>(null);
  let saveBanner = $state('');

  const SECTION_META: Record<NavSection, { title: string; subtitle: string }> = {
    layout: {
      title: 'Раскладка и опечатки',
      subtitle: 'Детерминированное автопереключение RU ↔ EN, CapsLock и исправление опечаток',
    },
    dictionary: {
      title: 'Словарь',
      subtitle: 'Белый список защищённых терминов, аббревиатур и автообученных слов',
    },
    snippets: {
      title: 'Автоподмены',
      subtitle: 'Спецсимволы и текстовые сниппеты с поддержкой любой раскладки',
    },
    lists: {
      title: 'Списки и регистр',
      subtitle: 'Циклическая смена регистра, капитализация и продолжение списков по Enter',
    },
    apps: {
      title: 'Приложения',
      subtitle: 'Исключения по процессам и фиксация стартовой раскладки при смене фокуса',
    },
    screenshot: {
      title: 'Скриншотер',
      subtitle: 'Локальный захват экрана, аннотации, пикселизация и размытие по Гауссу',
    },
    hotkeys: {
      title: 'Горячие клавиши',
      subtitle: 'Глобальные сочетания клавиш и одиночные нажатия модификаторов',
    },
    system: {
      title: 'Тест-драйв и система',
      subtitle: 'Проверка движка в реальном времени, тема оформления и перенос настроек',
    },
  };

  onMount(async () => {
    initThemeListener();
    const [loadedCfg, loadedStatus] = await Promise.all([getConfig(), getRuntimeStatus()]);
    config = loadedCfg;
    status = loadedStatus;
    applyTheme(loadedCfg.system.theme);
  });

  async function handleUpdate(next: KeyCraftConfig) {
    config = next;
    applyTheme(next.system.theme);
    try {
      const saved = await saveConfig(next);
      config = saved;
      saveBanner = 'Сохранено';
      setTimeout(() => {
        if (saveBanner === 'Сохранено') saveBanner = '';
      }, 1200);
    } catch (err) {
      saveBanner = err instanceof Error ? err.message : 'Ошибка сохранения';
    }
  }

  function handleToggleEnabled() {
    const next = cloneConfig(config);
    next.enabled = !next.enabled;
    handleUpdate(next);
  }
</script>

<div class="shell">
  <Sidebar
    active={activeSection}
    enabled={config.enabled}
    onselect={(s) => (activeSection = s)}
    ontoggleEnabled={handleToggleEnabled}
  />

  <main class="workspace">
    <header class="sticky-header">
      <div class="header-copy">
        <h1 class="section-title">{SECTION_META[activeSection].title}</h1>
        <p class="section-sub">{SECTION_META[activeSection].subtitle}</p>
      </div>
      {#if saveBanner}
        <span class="save-chip">{saveBanner}</span>
      {/if}
    </header>

    <div class="content-scroll">
      <div class="content-inner">
        {#if activeSection === 'layout'}
          <LayoutTyposView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'dictionary'}
          <DictionaryView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'snippets'}
          <SnippetsView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'lists'}
          <ListsCaseView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'apps'}
          <AppRulesView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'screenshot'}
          <ScreenshotView {config} {status} onupdate={handleUpdate} />
        {:else if activeSection === 'hotkeys'}
          <HotkeysView {config} onupdate={handleUpdate} />
        {:else if activeSection === 'system'}
          <SandboxSystemView {config} {status} onupdate={handleUpdate} />
        {/if}
      </div>
    </div>
  </main>
</div>

<style>
  .shell {
    display: flex;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-content);
  }

  .workspace {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .sticky-header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 56px;
    padding: 10px 24px;
    background: var(--bg-content);
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-copy {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .section-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .section-sub {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.3;
  }

  .save-chip {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--semantic-success);
    background: var(--semantic-success-soft);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px 28px;
  }

  .content-inner {
    max-width: 620px;
    margin: 0 auto;
  }
</style>
