<script lang="ts">
  import { getStatusLabel } from '$lib/utils';

  interface Props {
    status: string;
  }

  let { status }: Props = $props();

  const statusConfig = $derived.by(() => {
    switch (status) {
      case 'ACTIVE':
        return { color: 'text-[var(--success)]', bg: 'bg-[var(--success-subtle)]', dot: 'bg-[var(--success)]' };
      case 'CHECKING':
        return { color: 'text-[var(--accent)]', bg: 'bg-[var(--accent-subtle)]', dot: 'bg-[var(--accent)]' };
      case 'ERROR':
      case 'BLOCKED':
        return { color: 'text-[var(--danger)]', bg: 'bg-[var(--danger-subtle)]', dot: 'bg-[var(--danger)]' };
      case 'CAPTCHA_REQUIRED':
        return { color: 'text-[var(--warning)]', bg: 'bg-[var(--warning-subtle)]', dot: 'bg-[var(--warning)]' };
      case 'DISABLED':
        return { color: 'text-[var(--fg-muted)]', bg: 'bg-[var(--bg-subtle)]', dot: 'bg-[var(--fg-muted)]' };
      default:
        return { color: 'text-[var(--fg-muted)]', bg: 'bg-[var(--bg-subtle)]', dot: 'bg-[var(--fg-muted)]' };
    }
  });
</script>

  <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[12px] font-medium {statusConfig.color} {statusConfig.bg}">
  <span class="w-1.5 h-1.5 rounded-full {statusConfig.dot} {status === 'CHECKING' ? 'animate-pulse' : ''}"></span>
  {getStatusLabel(status)}
</span>
