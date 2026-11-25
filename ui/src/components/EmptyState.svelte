<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description: string;
    icon?: Snippet;
    action?: Snippet;
  }

  let { title, description, icon, action }: Props = $props();
</script>

<div class="empty">
  {#if icon}
    <div class="empty-icon">
      {@render icon()}
    </div>
  {/if}
  <div class="empty-title">{title}</div>
  <div class="empty-desc">{description}</div>
  {#if action}
    <div class="empty-action">
      {@render action()}
    </div>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 36px 24px;
    gap: 6px;
  }

  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--bg-control);
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .empty-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .empty-desc {
    max-width: 340px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .empty-action {
    margin-top: 8px;
  }
</style>
