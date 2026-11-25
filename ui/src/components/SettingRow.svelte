<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    children: Snippet;
  }

  let { title, description = '', children }: Props = $props();
</script>

<div class="setting-row">
  <div class="row-copy">
    <div class="row-title">{title}</div>
    {#if description}
      <div class="row-desc">{description}</div>
    {/if}
  </div>
  <div class="row-control">
    {@render children()}
  </div>
</div>

<style>
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    min-height: 48px;
  }

  .row-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .row-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.35;
  }

  .row-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.35;
  }

  .row-control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
