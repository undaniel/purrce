<script lang="ts">
  import { t } from '$lib/i18n';
  import Skeleton from './Skeleton.svelte';

  interface Props {
    rows?: number;
  }

  let { rows = 5 }: Props = $props();
</script>

<div class="card overflow-hidden">
  <div class="overflow-x-auto">
    <table class="w-full">
      <thead>
        <tr class="border-b border-[var(--border-subtle)]">
          <th class="table-head text-left px-4 py-3.5">{$t('table.product')}</th>
          <th class="table-head text-left px-4 py-3.5 hidden sm:table-cell">{$t('table.store')}</th>
          <th class="table-head text-right px-4 py-3.5">{$t('table.price')}</th>
          <th class="table-head text-right px-4 py-3.5 hidden md:table-cell">{$t('table.change')}</th>
          <th class="table-head text-right px-4 py-3.5 hidden lg:table-cell">{$t('table.trend')}</th>
          <th class="table-head text-right px-4 py-3.5">{$t('table.time')}</th>
        </tr>
      </thead>
      <tbody>
        {#each Array(rows) as _, i}
          <tr class="border-b border-[var(--border-subtle)] last:border-b-0">
            <td class="px-4 py-3">
              <div class="flex items-center gap-3">
                <Skeleton width="36px" height="36px" rounded="var(--radius-md)" />
                <div class="space-y-1.5">
                  <Skeleton width="140px" height="13px" />
                  <Skeleton width="60px" height="10px" />
                </div>
              </div>
            </td>
            <td class="px-4 py-3 hidden sm:table-cell">
              <Skeleton width="80px" height="12px" />
            </td>
            <td class="px-4 py-3 text-right">
              <Skeleton width="90px" height="14px" rounded="var(--radius-sm)" />
            </td>
            <td class="px-4 py-3 text-right hidden md:table-cell">
              <Skeleton width="56px" height="18px" rounded="var(--radius-sm)" />
            </td>
            <td class="px-4 py-3 text-right hidden lg:table-cell">
              <Skeleton width="56px" height="20px" rounded="var(--radius-sm)" />
            </td>
            <td class="px-4 py-3 text-right">
              <Skeleton width="60px" height="12px" rounded="var(--radius-sm)" />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
