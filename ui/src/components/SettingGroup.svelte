<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    children: Snippet;
  }

  let { title = '', children }: Props = $props();
</script>

<section class="group">
  {#if title}
    <h3 class="group-title">{title}</h3>
  {/if}
  <div class="group-card">
    {@render children()}
  </div>
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-title {
    margin: 0;
    padding: 0 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .group-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-subtle);
    overflow: hidden;
  }

  .group-card :global(> .setting-row + .setting-row) {
    border-top: 1px solid var(--border-subtle);
  }
</style>
