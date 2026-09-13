<script lang="ts">
  import { t } from '$lib/i18n';

  interface Props {
    label?: string;
  }
  let { label }: Props = $props();
</script>

<div class="flex items-center justify-center py-20">
  <div class="flex items-center gap-2.5 text-[13px] text-[var(--fg-muted)]">
    <span class="relative flex h-3.5 w-3.5">
      <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--accent)] opacity-30"></span>
      <span class="relative inline-flex rounded-full h-3.5 w-3.5 bg-[var(--accent)]"></span>
    </span>
    {label ?? $t('loading.default')}
  </div>
</div>
