<script lang="ts">
  interface Props {
    store: string;
  }

  let { store }: Props = $props();
</script>

  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-[var(--bg-subtle)] text-[var(--fg-muted)]">
  {store}
</span>
