<script lang="ts">
  import { Camera, AppWindow, Maximize } from 'lucide-svelte';
  import SettingGroup from '../components/SettingGroup.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import Toggle from '../components/Toggle.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import KeyRecorder from '../components/KeyRecorder.svelte';
  import { cloneConfig, createDefaultConfig, triggerScreenshot } from '../lib/ipc';
  import type { KeyCraftConfig, RuntimeStatus } from '../lib/types';

  interface Props {
    config: KeyCraftConfig;
    status: RuntimeStatus | null;
    onupdate: (next: KeyCraftConfig) => void;
  }

  let { config, status, onupdate }: Props = $props();

  const PALETTE = ['#DC2626', '#D97706', '#16A34A', '#2563EB', '#18181B', '#FFFFFF'];
  const DEFAULT_HOTKEYS = createDefaultConfig().hotkeys;

  function patch(mutator: (draft: KeyCraftConfig) => void) {
    const next = cloneConfig(config);
    mutator(next);
    onupdate(next);
  }
</script>

<div class="view-stack">
  <SettingGroup title="Модуль скриншотера">
    <SettingRow
      title="Включить встроенный скриншотер"
      description="Отключите, чтобы KeyCraft не перехватывал PrintScreen и не конфликтовал с Ножницами Windows"
    >
      <Toggle
        checked={config.screenshot.enabled}
        ariaLabel="Включить встроенный скриншотер"
        onchange={(v) => patch((c) => (c.screenshot.enabled = v))}
      />
    </SettingRow>
  </SettingGroup>

  <div class="shot-body" class:disabled={!config.screenshot.enabled}>
  <div class="quick-capture">
    <button type="button" class="btn btn-primary" disabled={!config.screenshot.enabled} onclick={() => triggerScreenshot('region')}>
      <Camera size={14} strokeWidth={1.5} />
      Снять область
    </button>
    <button type="button" class="btn" disabled={!config.screenshot.enabled} onclick={() => triggerScreenshot('window')}>
      <AppWindow size={14} strokeWidth={1.5} />
      Активное окно
    </button>
    <button type="button" class="btn" disabled={!config.screenshot.enabled} onclick={() => triggerScreenshot('fullscreen')}>
      <Maximize size={14} strokeWidth={1.5} />
      Весь экран
    </button>
  </div>

  <SettingGroup title="Захват и аннотации">
    <SettingRow
      title="Лупа-прицел по пикселям"
      description="Показывает увеличенный фрагмент под курсором и размеры выделения W × H"
    >
      <Toggle
        checked={config.screenshot.show_magnifier}
        ariaLabel="Лупа-прицел по пикселям"
        onchange={(v) => patch((c) => (c.screenshot.show_magnifier = v))}
      />
    </SettingRow>

    <SettingRow
      title="Копировать в буфер после сохранения PNG"
      description="При сохранении файла через Ctrl+S изображение также помещается в буфер обмена"
    >
      <Toggle
        checked={config.screenshot.copy_after_save}
        ariaLabel="Копировать в буфер после сохранения PNG"
        onchange={(v) => patch((c) => (c.screenshot.copy_after_save = v))}
      />
    </SettingRow>

    <SettingRow title="Цвет аннотаций по умолчанию" description="6 спокойных читаемых цветов без неоновых оттенков">
      <div class="swatches">
        {#each PALETTE as color}
          <button
            type="button"
            class="swatch"
            class:selected={config.screenshot.default_color === color}
            style={`background:${color}`}
            aria-label={`Цвет ${color}`}
            onclick={() => patch((c) => (c.screenshot.default_color = color))}
          ></button>
        {/each}
      </div>
    </SettingRow>

    <SettingRow title="Толщина линий по умолчанию" description="Применяется к стрелкам, рамкам, линиям и эллипсам">
      <SegmentedControl
        options={[
          { value: '2', label: 'Тонкая (2px)' },
          { value: '3', label: 'Средняя (3px)' },
          { value: '5', label: 'Толстая (5px)' },
        ]}
        value={String(config.screenshot.default_stroke_width)}
        onchange={(v) => patch((c) => (c.screenshot.default_stroke_width = Number(v)))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Сохранение файлов">
    <SettingRow
      title="Папка для PNG-снимков"
      description={status ? `По умолчанию: ${status.default_screenshots_dir}` : 'Оставьте пустой для Изображения\\KeyCraft'}
    >
      <input
        type="text"
        class="dir-input mono"
        value={config.screenshot.save_dir}
        placeholder="По умолчанию"
        onchange={(e) =>
          patch((c) => (c.screenshot.save_dir = (e.target as HTMLInputElement).value))}
      />
    </SettingRow>

    <SettingRow title="Шаблон имени файла" description="Поддерживаются директивы даты strftime (%Y, %m, %d, %H, %M, %S)">
      <input
        type="text"
        class="dir-input mono"
        value={config.screenshot.filename_pattern}
        onchange={(e) =>
          patch((c) => (c.screenshot.filename_pattern = (e.target as HTMLInputElement).value))}
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="Горячие клавиши скриншотера">
    <SettingRow title="Захват области" description="При включённом скриншотере PrintScreen перехватывается вместо Ножниц Windows">
      <KeyRecorder
        value={config.hotkeys.screenshot_region}
        defaultValue={DEFAULT_HOTKEYS.screenshot_region}
        onchange={(v) => patch((c) => (c.hotkeys.screenshot_region = v))}
      />
    </SettingRow>
    <SettingRow title="Захват активного окна" description="Автоматическое выделение границ окна без невидимой тени Windows">
      <KeyRecorder
        value={config.hotkeys.screenshot_window}
        defaultValue={DEFAULT_HOTKEYS.screenshot_window}
        onchange={(v) => patch((c) => (c.hotkeys.screenshot_window = v))}
      />
    </SettingRow>
    <SettingRow title="Захват всего экрана" description="Полный снимок виртуального рабочего стола">
      <KeyRecorder
        value={config.hotkeys.screenshot_fullscreen}
        defaultValue={DEFAULT_HOTKEYS.screenshot_fullscreen}
        onchange={(v) => patch((c) => (c.hotkeys.screenshot_fullscreen = v))}
      />
    </SettingRow>
  </SettingGroup>
  </div>
</div>

<style>
  .view-stack {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .shot-body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    transition: opacity var(--dur-fast) var(--ease);
  }

  .shot-body.disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  .quick-capture {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .swatches {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 20px;
    height: 20px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    cursor: pointer;
    padding: 0;
    transition: transform var(--dur-fast) var(--ease);
  }

  .swatch:hover {
    transform: scale(1.08);
  }

  .swatch.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .dir-input {
    width: 230px;
    height: 28px;
    font-size: 12px;
  }
</style>
