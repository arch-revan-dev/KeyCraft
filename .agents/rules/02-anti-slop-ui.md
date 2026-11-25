# KeyCraft — Anti-Slop Design Contract

KeyCraft is a precision desktop utility engineered to the craft standards of **Linear, Raycast, Things 3, and native macOS / Windows 11 Fluent Settings**. Every screen, modal, control, and micro-interaction MUST obey this contract.

## 1. Strictly Forbidden (AI-Slop Blacklist)
- **Zero Emojis:** Never place emojis (`🚀`, `✨`, `⚡`, `🔥`, `🎯`, `📋`, etc.) anywhere in labels, headings, buttons, tooltips, empty states, or logs. Use clean monochrome stroke icons (`lucide-svelte`, `strokeWidth={1.5}`) exclusively.
- **Zero Neon / Purple Gradients or Glow Shadows:** No purple/blue-pink gradients, no colored `box-shadow` glow effects, no heavy glassmorphism for decoration, no default Tailwind slate-blue (`#0F172A` / `#1E293B`).
- **Zero Marketing Copy or Fluff:** Never write hype ("Ускорьте вашу продуктивность на максимум!", "Умный помощник нового поколения"). Write crisp, calm, technical Russian copy (Apple HIG / Linear / Birman style): direct, informative, 1 line per description.
- **Zero Decorative Dashboards:** No fake charts, empty stat cards, hero banners, or oversized padding. This is a focused utility, not a SaaS landing page.
- **Zero Bloated UI Kits or External Assets:** Pure CSS with Design Tokens (`ui/src/styles/tokens.css`). No Tailwind, MUI, Chakra, or external font/CDN links.

## 2. Color System (CSS Custom Properties Only)
Support automatic system theme (`prefers-color-scheme`) plus manual override (`Система` / `Светлая` / `Тёмная`):

### Light Theme — Warm Studio Paper
- `--bg-sidebar: #F5F5F4`
- `--bg-content: #FAFAF9`
- `--bg-surface: #FFFFFF`
- `--bg-surface-hover: #F4F4F5`
- `--bg-control: #F0F0EE`
- `--border-subtle: rgba(0, 0, 0, 0.07)`
- `--border-strong: rgba(0, 0, 0, 0.13)`
- `--text-primary: #18181B`
- `--text-secondary: #636369`
- `--text-tertiary: #9CA3AF`
- `--accent: #2563EB`
- `--accent-hover: #1D4ED8`
- `--accent-soft: rgba(37, 99, 235, 0.10)`
- `--semantic-success: #15803D`
- `--semantic-danger: #B91C1C`

### Dark Theme — Deep Tactile Graphite
- `--bg-sidebar: #161618`
- `--bg-content: #1C1C1F`
- `--bg-surface: #232327`
- `--bg-surface-hover: #2A2A2F`
- `--bg-control: #2C2C31`
- `--border-subtle: rgba(255, 255, 255, 0.08)`
- `--border-strong: rgba(255, 255, 255, 0.15)`
- `--text-primary: #F4F4F5`
- `--text-secondary: #A1A1AA`
- `--text-tertiary: #71717A`
- `--accent: #4F8BFF`
- `--accent-hover: #6BA0FF`
- `--accent-soft: rgba(79, 139, 255, 0.14)`
- `--semantic-success: #4ADE80`
- `--semantic-danger: #F87171`

**Accent Discipline:** `--accent` is reserved strictly for active toggles, keyboard focus rings, active navigation indicators, and primary action buttons.

## 3. Typography & Modular Grid
- **UI Font:** `-apple-system, BlinkMacSystemFont, "Segoe UI Variable", "Segoe UI", sans-serif` with `font-feature-settings: "cv02", "cv03", "cv04", "cv11"` and `font-variant-numeric: tabular-nums` on all numbers, dimensions, latency values, and counts.
- **Monospace Font:** `"SF Mono", "Cascadia Code", "JetBrains Mono", monospace` for triggers, hotkeys, process names, and text examples.
- **4px Modular Grid:** All spacing, paddings, radii, and dimensions are multiples of `4px`.
- **Settings Window Layout:** `860 × 600 px`, fixed left sidebar `210px`, scrollable right workspace with a sticky `48px` section header (`1px` bottom border).

## 4. Tactile Details (Craft)
- **Grouped Inset Lists:** Settings live inside bordered surface groups (`border-radius: 8px`, `1px solid var(--border-subtle)`) with `1px` internal row dividers. Each row pairs Title (`13px`, weight `500`) + 1-line description (`12px`, `--text-secondary`) on the left with a crisp control (Toggle, SegmentedControl, Select, Input, KeyRecorder) on the right.
- **Keycap `<kbd>` Component:** Realistic tactile keycap with `1px solid var(--border-strong)`, subtle `1px` bottom lip (`box-shadow: 0 1px 0 var(--border-strong)`), monospace `11px`, crisp labels (`Ctrl`, `Alt`, `Shift`, `Win`, `Esc`, `Pause`), and clear live-recording state («Нажмите сочетание…»).
- **Micro-Interactions:** Fast, crisp transitions (`120ms–150ms cubic-bezier(0.2, 0, 0, 1)`). Never sluggish or bouncy.
- **Empty States:** Clean, understated text explanation + single clear action button inside the inset container. No oversized clip-art illustrations.
- **Screenshotter Palette:** 6 calm, legible annotation colors only (`#DC2626`, `#D97706`, `#16A34A`, `#2563EB`, `#18181B`, `#FFFFFF`).
