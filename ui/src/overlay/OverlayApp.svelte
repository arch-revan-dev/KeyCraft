<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ArrowUpRight,
    Square,
    Circle,
    Minus,
    Highlighter,
    Type,
    Hash,
    Grid2x2,
    Droplets,
    Undo2,
    Redo2,
    Copy,
    Save,
    Pin,
    X,
  } from 'lucide-svelte';
  import {
    getCaptureImageDataUrl,
    getCaptureMetadata,
    isTauriRuntime,
    screenshotClose,
    screenshotCopyPng,
    screenshotPinPng,
    screenshotSavePng,
  } from '../lib/ipc';
  import type { CaptureMetadata } from '../lib/types';
  import {
    CALM_COLORS,
    drawAnnotation,
    exportSelectionToPng,
    normalizeRect,
    type AnnotationItem,
    type AnnotationTool,
    type SelectionBox,
  } from './canvasEngine';

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let loupeEl = $state<HTMLCanvasElement | null>(null);
  let bgSource: HTMLImageElement | HTMLCanvasElement | null = null;
  let scaleX = $state(1);
  let scaleY = $state(1);

  let meta = $state<CaptureMetadata | null>(null);
  let selection = $state<SelectionBox | null>(null);
  let phase = $state<'selecting' | 'annotating'>('selecting');
  let activeTool = $state<AnnotationTool>('arrow');
  let activeColor = $state<string>('#DC2626');
  let strokeWidth = $state<number>(3);

  let annotations = $state<AnnotationItem[]>([]);
  let redoStack = $state<AnnotationItem[]>([]);
  let draftItem = $state<AnnotationItem | null>(null);
  let nextId = 1;
  let stepCounter = 1;

  let draggingSelection = false;
  let dragStart = { x: 0, y: 0 };
  let cursor = $state({ x: 0, y: 0, visible: false });

  let textPrompt = $state<{ x: number; y: number; value: string } | null>(null);
  let textInputEl = $state<HTMLInputElement | null>(null);
  let toolbarWidth = $state(680);

  $effect(() => {
    if (textPrompt && textInputEl) {
      textInputEl.focus();
    }
  });

  const TOOLS: { id: AnnotationTool; label: string; icon: typeof ArrowUpRight }[] = [
    { id: 'arrow', label: 'Стрелка', icon: ArrowUpRight },
    { id: 'rect', label: 'Прямоугольник', icon: Square },
    { id: 'ellipse', label: 'Эллипс', icon: Circle },
    { id: 'line', label: 'Линия', icon: Minus },
    { id: 'highlighter', label: 'Маркер', icon: Highlighter },
    { id: 'text', label: 'Текст', icon: Type },
    { id: 'step', label: 'Шаг 1, 2, 3', icon: Hash },
    { id: 'pixelate', label: 'Пикселизация', icon: Grid2x2 },
    { id: 'blur', label: 'Размытие (Gaussian Blur)', icon: Droplets },
  ];

  function createDevPreviewBackdrop(w: number, h: number): HTMLCanvasElement {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    const ctx = c.getContext('2d')!;
    ctx.fillStyle = '#EAEAE7';
    ctx.fillRect(0, 0, w, h);

    // Subtle studio desktop mockup for visual inspection in dev server
    ctx.fillStyle = '#FFFFFF';
    ctx.strokeStyle = 'rgba(0,0,0,0.1)';
    ctx.lineWidth = 1;
    ctx.fillRect(80, 60, w - 160, h - 120);
    ctx.strokeRect(80, 60, w - 160, h - 120);

    ctx.fillStyle = '#18181B';
    ctx.font = '600 18px -apple-system, BlinkMacSystemFont, "Segoe UI Variable", sans-serif';
    ctx.fillText('KeyCraft Screenshot Preview Canvas', 112, 108);
    ctx.fillStyle = '#636369';
    ctx.font = '13px "Cascadia Code", monospace';
    ctx.fillText('api_token = "kc_demo_49a8f12c9b0e"   // Попробуйте размытие или пикселизацию', 112, 144);
    ctx.fillText('email     = "team@keycraft.local"', 112, 170);
    return c;
  }

  onMount(() => {
    (async () => {
      const m = await getCaptureMetadata();
      meta = m;
      activeColor = m.default_color || '#DC2626';
      strokeWidth = m.default_stroke_width || 3;

      const cssW = window.innerWidth;
      const cssH = window.innerHeight;
      const dpr = window.devicePixelRatio || 1;

      if (isTauriRuntime()) {
        const dataUrl = await getCaptureImageDataUrl();
        const img = new Image();
        img.onload = () => {
          bgSource = img;
          const physW = img.naturalWidth || Math.round(cssW * dpr);
          const physH = img.naturalHeight || Math.round(cssH * dpr);
          if (canvasEl) {
            canvasEl.width = physW;
            canvasEl.height = physH;
          }
          scaleX = physW / Math.max(1, cssW);
          scaleY = physH / Math.max(1, cssH);
          initSelectionForMode(m, cssW, cssH, scaleX, scaleY);
          redraw();
        };
        img.src = dataUrl || `http://keycraft-capture.localhost/frame?t=${Date.now()}`;
      } else {
        const physW = Math.round(cssW * dpr);
        const physH = Math.round(cssH * dpr);
        if (canvasEl) {
          canvasEl.width = physW;
          canvasEl.height = physH;
        }
        scaleX = physW / Math.max(1, cssW);
        scaleY = physH / Math.max(1, cssH);
        bgSource = createDevPreviewBackdrop(physW, physH);
        selection = { x: 96, y: 74, w: Math.min(540, cssW - 192), h: Math.min(240, cssH - 200) };
        phase = 'annotating';
        redraw();
      }
    })();

    const onKey = (e: KeyboardEvent) => {
      if (textPrompt) return;
      const mod = e.ctrlKey || e.metaKey;
      const isZ = e.code === 'KeyZ' || e.key.toLowerCase() === 'z';
      const isY = e.code === 'KeyY' || e.key.toLowerCase() === 'y';
      const isC = e.code === 'KeyC' || e.key.toLowerCase() === 'c';
      const isS = e.code === 'KeyS' || e.key.toLowerCase() === 's';

      if (e.key === 'Escape') {
        e.preventDefault();
        screenshotClose();
      } else if (mod && isZ) {
        e.preventDefault();
        if (e.shiftKey) redo();
        else undo();
      } else if (mod && isY) {
        e.preventDefault();
        redo();
      } else if (e.key === 'Enter' || (mod && isC)) {
        if (selection && selection.w > 4 && selection.h > 4) {
          e.preventDefault();
          handleCopy();
        }
      } else if (mod && isS) {
        if (selection && selection.w > 4 && selection.h > 4) {
          e.preventDefault();
          handleSave();
        }
      }
    };

    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  function initSelectionForMode(
    m: CaptureMetadata,
    cssW: number,
    cssH: number,
    sx: number,
    sy: number
  ) {
    if (m.mode === 'fullscreen') {
      selection = { x: 0, y: 0, w: cssW, h: cssH };
      phase = 'annotating';
    } else if (m.mode === 'window' && m.active_window_rect) {
      // Convert physical DWM extended frame bounds cleanly into CSS logical pixels
      const r = m.active_window_rect;
      const rawX = (r.x - m.origin_x) / sx;
      const rawY = (r.y - m.origin_y) / sy;
      const rawW = r.width / sx;
      const rawH = r.height / sy;
      const x = Math.max(0, Math.min(cssW - 16, rawX));
      const y = Math.max(0, Math.min(cssH - 16, rawY));
      const right = Math.min(cssW, rawX + rawW);
      const bottom = Math.min(cssH, rawY + rawH);
      selection = {
        x,
        y,
        w: Math.max(16, right - x),
        h: Math.max(16, bottom - y),
      };
      phase = 'annotating';
    }
  }

  function redraw() {
    if (!canvasEl || !bgSource) return;
    const ctx = canvasEl.getContext('2d');
    if (!ctx) return;

    const physW = canvasEl.width;
    const physH = canvasEl.height;
    const cssW = physW / scaleX;
    const cssH = physH / scaleY;

    // 1. Draw desktop capture at 1:1 native physical pixel resolution (zero downscaling/blur)
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, physW, physH);
    ctx.drawImage(bgSource, 0, 0, physW, physH);

    // 2. Switch to CSS logical coordinates scaled by DPR for crisp overlay masks & annotations
    ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
    ctx.imageSmoothingEnabled = true;

    // Dim outside selected area
    ctx.fillStyle = 'rgba(12, 12, 14, 0.48)';
    if (!selection || selection.w < 2 || selection.h < 2) {
      ctx.fillRect(0, 0, cssW, cssH);
    } else {
      const s = selection;
      ctx.fillRect(0, 0, cssW, s.y);
      ctx.fillRect(0, s.y + s.h, cssW, cssH - (s.y + s.h));
      ctx.fillRect(0, s.y, s.x, s.h);
      ctx.fillRect(s.x + s.w, s.y, cssW - (s.x + s.w), s.h);

      // Render annotations clipped inside selection
      ctx.save();
      ctx.beginPath();
      ctx.rect(s.x, s.y, s.w, s.h);
      ctx.clip();
      for (const item of annotations) {
        drawAnnotation(ctx, item, 0, 0, scaleX, scaleY);
      }
      if (draftItem) {
        drawAnnotation(ctx, draftItem, 0, 0, scaleX, scaleY);
      }
      ctx.restore();

      // Crisp 1px selection border
      ctx.strokeStyle = '#4F8BFF';
      ctx.lineWidth = 1.5;
      ctx.strokeRect(s.x + 0.5, s.y + 0.5, s.w - 1, s.h - 1);
    }

    if (phase === 'selecting' && meta?.show_magnifier && cursor.visible) {
      drawMagnifier();
    }
  }

  function drawMagnifier() {
    if (!loupeEl || !bgSource) return;
    const lctx = loupeEl.getContext('2d');
    if (!lctx) return;
    const size = 104;
    const zoom = 6;
    const sample = Math.floor(size / zoom);
    // Sample directly from physical desktop pixels
    const sx = Math.round(cursor.x * scaleX - sample / 2);
    const sy = Math.round(cursor.y * scaleY - sample / 2);

    lctx.imageSmoothingEnabled = false;
    lctx.clearRect(0, 0, size, size);
    lctx.drawImage(bgSource, sx, sy, sample, sample, 0, 0, size, size);

    // Pixel crosshair
    lctx.strokeStyle = 'rgba(79, 139, 255, 0.9)';
    lctx.lineWidth = 1;
    const mid = size / 2;
    lctx.beginPath();
    lctx.moveTo(mid, 0);
    lctx.lineTo(mid, size);
    lctx.moveTo(0, mid);
    lctx.lineTo(size, mid);
    lctx.stroke();
  }

  function pointInSelection(x: number, y: number): boolean {
    if (!selection) return false;
    return x >= selection.x && x <= selection.x + selection.w && y >= selection.y && y <= selection.y + selection.h;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    if (textPrompt) {
      commitTextPrompt();
      return;
    }
    const x = e.clientX;
    const y = e.clientY;

    if (phase === 'selecting') {
      draggingSelection = true;
      dragStart = { x, y };
      selection = { x, y, w: 0, h: 0 };
      redraw();
      return;
    }

    if (!pointInSelection(x, y)) return;

    if (activeTool === 'step') {
      annotations = [
        ...annotations,
        {
          id: nextId++,
          tool: 'step',
          color: activeColor,
          strokeWidth,
          x1: x,
          y1: y,
          x2: x,
          y2: y,
          stepNumber: stepCounter++,
        },
      ];
      redoStack = [];
      redraw();
      return;
    }

    if (activeTool === 'text') {
      textPrompt = { x, y, value: '' };
      return;
    }

    draftItem = {
      id: nextId++,
      tool: activeTool,
      color: activeColor,
      strokeWidth,
      x1: x,
      y1: y,
      x2: x,
      y2: y,
      points: activeTool === 'highlighter' ? [{ x, y }] : undefined,
    };
    redraw();
  }

  function onPointerMove(e: PointerEvent) {
    cursor = { x: e.clientX, y: e.clientY, visible: true };
    if (draggingSelection) {
      selection = normalizeRect(dragStart.x, dragStart.y, e.clientX, e.clientY);
      redraw();
      return;
    }
    if (draftItem) {
      draftItem.x2 = e.clientX;
      draftItem.y2 = e.clientY;
      if (draftItem.points) {
        draftItem.points.push({ x: e.clientX, y: e.clientY });
      }
      redraw();
      return;
    }
    if (phase === 'selecting') {
      redraw();
    }
  }

  function onPointerUp() {
    if (draggingSelection) {
      draggingSelection = false;
      if (selection && selection.w >= 8 && selection.h >= 8) {
        phase = 'annotating';
      } else {
        selection = null;
      }
      redraw();
      return;
    }

    if (draftItem) {
      annotations = [...annotations, draftItem];
      redoStack = [];
      draftItem = null;
      redraw();
    }
  }

  function commitTextPrompt() {
    if (!textPrompt) return;
    const val = textPrompt.value.trim();
    if (val) {
      annotations = [
        ...annotations,
        {
          id: nextId++,
          tool: 'text',
          color: activeColor,
          strokeWidth,
          x1: textPrompt.x,
          y1: textPrompt.y,
          x2: textPrompt.x,
          y2: textPrompt.y,
          text: val,
        },
      ];
      redoStack = [];
    }
    textPrompt = null;
    redraw();
  }

  function undo() {
    if (annotations.length === 0) return;
    const popped = annotations[annotations.length - 1];
    annotations = annotations.slice(0, -1);
    redoStack = [...redoStack, popped];
    if (popped.tool === 'step' && stepCounter > 1) stepCounter -= 1;
    redraw();
  }

  function redo() {
    if (redoStack.length === 0) return;
    const item = redoStack[redoStack.length - 1];
    redoStack = redoStack.slice(0, -1);
    annotations = [...annotations, item];
    if (item.tool === 'step') stepCounter += 1;
    redraw();
  }

  function renderExport(): string | null {
    if (!bgSource || !selection || selection.w < 2 || selection.h < 2) return null;
    return exportSelectionToPng(bgSource, selection, annotations, scaleX, scaleY);
  }

  async function handleCopy() {
    commitTextPrompt();
    const png = renderExport();
    if (png) await screenshotCopyPng(png);
  }

  async function handleSave() {
    commitTextPrompt();
    const png = renderExport();
    if (png) await screenshotSavePng(png);
  }

  async function handlePin() {
    commitTextPrompt();
    const png = renderExport();
    if (!png || !selection) return;
    const sx = (meta?.origin_x ?? 0) + Math.round(selection.x * scaleX);
    const sy = (meta?.origin_y ?? 0) + Math.round(selection.y * scaleY);
    await screenshotPinPng(png, sx, sy);
  }

  const toolbarPos = $derived.by(() => {
    if (!selection) return { left: 12, top: 12 };
    const vw = typeof window !== 'undefined' ? window.innerWidth : 1280;
    const vh = typeof window !== 'undefined' ? window.innerHeight : 720;
    const barW = Math.max(toolbarWidth, 620);
    const left = Math.max(8, Math.min(vw - barW - 8, selection.x));
    const below = selection.y + selection.h + 10;
    const top = below + 44 < vh ? below : Math.max(8, selection.y - 46);
    return { left, top };
  });
</script>

<div
  class="overlay-root"
  role="application"
  aria-label="Редактор скриншота"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
>
  <canvas bind:this={canvasEl} class="stage"></canvas>

  {#if selection && selection.w > 4 && selection.h > 4}
    <div
      class="dim-pill mono tabular"
      style={`left:${Math.max(8, selection.x)}px;top:${Math.max(8, selection.y - 26)}px`}
    >
      {Math.round(selection.w * scaleX)} × {Math.round(selection.h * scaleY)}
    </div>
  {/if}

  {#if phase === 'selecting' && meta?.show_magnifier && cursor.visible}
    <div
      class="loupe"
      style={`left:${Math.min(window.innerWidth - 124, cursor.x + 18)}px;top:${Math.min(window.innerHeight - 134, cursor.y + 18)}px`}
    >
      <canvas bind:this={loupeEl} width={104} height={104}></canvas>
      <div class="loupe-coords mono tabular">
        {Math.round(cursor.x * scaleX)}, {Math.round(cursor.y * scaleY)}
      </div>
    </div>
  {/if}

  {#if textPrompt}
    <input
      bind:this={textInputEl}
      type="text"
      class="inline-text-input"
      style={`left:${textPrompt.x}px;top:${textPrompt.y}px;color:${activeColor}`}
      bind:value={textPrompt.value}
      placeholder="Текст…"
      onpointerdown={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          e.stopPropagation();
          commitTextPrompt();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          textPrompt = null;
        }
      }}
      onblur={commitTextPrompt}
    />
  {/if}

  {#if phase === 'annotating' && selection}
    <div
      class="toolbar"
      role="toolbar"
      aria-label="Инструменты аннотаций"
      tabindex="-1"
      bind:clientWidth={toolbarWidth}
      style={`left:${toolbarPos.left}px;top:${toolbarPos.top}px`}
      onpointerdown={(e) => e.stopPropagation()}
    >
      <div class="tool-group">
        {#each TOOLS as t (t.id)}
          {@const Icon = t.icon}
          <button
            type="button"
            class="tool-btn"
            class:active={activeTool === t.id}
            aria-pressed={activeTool === t.id}
            title={t.label}
            onclick={() => {
              commitTextPrompt();
              activeTool = t.id;
            }}
          >
            <Icon size={14} strokeWidth={1.5} />
          </button>
        {/each}
      </div>

      <span class="divider"></span>

      <div class="tool-group colors">
        {#each CALM_COLORS as c}
          <button
            type="button"
            class="color-dot"
            class:selected={activeColor === c}
            style={`background:${c}`}
            aria-label={`Цвет ${c}`}
            onclick={() => (activeColor = c)}
          ></button>
        {/each}
      </div>

      <span class="divider"></span>

      <div class="tool-group">
        {#each [2, 3, 5] as w}
          <button
            type="button"
            class="tool-btn width-btn"
            class:active={strokeWidth === w}
            title={`Толщина ${w}px`}
            onclick={() => (strokeWidth = w)}
          >
            <span class="width-dot" style={`width:${w + 3}px;height:${w + 3}px`}></span>
          </button>
        {/each}
      </div>

      <span class="divider"></span>

      <div class="tool-group">
        <button type="button" class="tool-btn" title="Отменить (Ctrl+Z)" disabled={annotations.length === 0} onclick={undo}>
          <Undo2 size={15} strokeWidth={1.5} />
        </button>
        <button type="button" class="tool-btn" title="Повторить (Ctrl+Shift+Z)" disabled={redoStack.length === 0} onclick={redo}>
          <Redo2 size={15} strokeWidth={1.5} />
        </button>
      </div>

      <span class="divider"></span>

      <div class="tool-group actions">
        <button type="button" class="tool-btn" title="Закрепить поверх окон" onclick={handlePin}>
          <Pin size={15} strokeWidth={1.5} />
        </button>
        <button type="button" class="tool-btn" title="Сохранить PNG (Ctrl+S)" onclick={handleSave}>
          <Save size={15} strokeWidth={1.5} />
        </button>
        <button type="button" class="tool-btn primary" title="Копировать (Enter / Ctrl+C)" onclick={handleCopy}>
          <Copy size={14} strokeWidth={1.5} />
          <span>Копировать</span>
        </button>
        <button type="button" class="tool-btn" title="Закрыть (Esc)" onclick={() => screenshotClose()}>
          <X size={15} strokeWidth={1.5} />
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .overlay-root {
    position: fixed;
    inset: 0;
    cursor: crosshair;
    user-select: none;
  }

  .stage {
    display: block;
    width: 100%;
    height: 100%;
  }

  .dim-pill {
    position: fixed;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(24, 24, 27, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: #f4f4f5;
    font-size: 11px;
    pointer-events: none;
  }

  .loupe {
    position: fixed;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.24);
    background: #18181b;
    pointer-events: none;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
  }

  .loupe canvas {
    display: block;
  }

  .loupe-coords {
    padding: 2px 6px;
    font-size: 10px;
    color: #a1a1aa;
    text-align: center;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
  }

  .inline-text-input {
    position: fixed;
    min-width: 140px;
    padding: 4px 8px;
    background: rgba(24, 24, 27, 0.88);
    border: 1px solid #4f8bff;
    border-radius: 4px;
    font-weight: 600;
    font-size: 14px;
  }

  .toolbar {
    position: fixed;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    border-radius: 8px;
    background: #1c1c1f;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.36);
    cursor: default;
    max-width: calc(100vw - 16px);
  }

  .tool-group {
    display: inline-flex;
    align-items: center;
    gap: 1px;
  }

  .colors {
    gap: 4px;
    padding: 0 2px;
  }

  .divider {
    width: 1px;
    height: 16px;
    background: rgba(255, 255, 255, 0.12);
    margin: 0 1px;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 26px;
    min-width: 26px;
    padding: 0 5px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: #d4d4d8;
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 120ms cubic-bezier(0.2, 0, 0, 1), color 120ms;
  }

  .tool-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
  }

  .tool-btn.active {
    background: rgba(79, 139, 255, 0.22);
    color: #4f8bff;
  }

  .tool-btn.primary {
    background: #2563eb;
    color: #ffffff;
    padding: 0 8px;
  }

  .tool-btn.primary:hover {
    background: #1d4ed8;
  }

  .tool-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .color-dot {
    width: 14px;
    height: 14px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.25);
    padding: 0;
    cursor: pointer;
  }

  .color-dot.selected {
    outline: 2px solid #4f8bff;
    outline-offset: 2px;
  }

  .width-dot {
    border-radius: 999px;
    background: currentColor;
  }
</style>
