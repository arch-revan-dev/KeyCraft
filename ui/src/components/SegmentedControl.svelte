<script lang="ts" generics="T extends string">
  interface Option<V extends string> {
    value: V;
    label: string;
  }

  interface Props {
    options: Option<T>[];
    value: T;
    onchange?: (val: T) => void;
  }

  let { options, value = $bindable(), onchange }: Props = $props();

  function select(next: T) {
    if (next === value) return;
    value = next;
    onchange?.(next);
  }
</script>

<div class="segmented" role="radiogroup">
  {#each options as opt (opt.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === opt.value}
      class="seg-btn"
      class:active={value === opt.value}
      onclick={() => select(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-md);
    background: var(--bg-control);
    border: 1px solid var(--border-subtle);
  }

  .seg-btn {
    height: 24px;
    padding: 0 10px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease),
      box-shadow var(--dur-fast) var(--ease);
  }

  .seg-btn:hover:not(.active) {
    color: var(--text-primary);
  }

  .seg-btn.active {
    background: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: var(--shadow-subtle);
  }
</style>
