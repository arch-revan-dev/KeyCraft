<script lang="ts">
  interface Props {
    checked: boolean;
    disabled?: boolean;
    ariaLabel?: string;
    onchange?: (value: boolean) => void;
  }

  let { checked = $bindable(false), disabled = false, ariaLabel = '', onchange }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  class="toggle"
  class:checked
  {disabled}
  onclick={toggle}
>
  <span class="thumb"></span>
</button>

<style>
  .toggle {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 34px;
    height: 20px;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--bg-control);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }

  .toggle.checked {
    background: var(--accent);
    border-color: var(--accent);
  }

  .toggle:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .toggle:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .thumb {
    width: 14px;
    height: 14px;
    border-radius: 999px;
    background: #ffffff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.22);
    transform: translateX(0);
    transition: transform var(--dur-fast) var(--ease);
  }

  .toggle.checked .thumb {
    transform: translateX(14px);
  }
</style>
