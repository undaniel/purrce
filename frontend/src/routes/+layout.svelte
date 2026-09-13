<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { page } from '$app/state';
  import { beforeNavigate, afterNavigate } from '$app/navigation';
  import { initTheme, toggleTheme } from '$lib/stores/theme';
  import { startEventStream, stopEventStream, lastEvent } from '$lib/stores/events';
  import { formatPrice } from '$lib/utils';
  import { t, initLocale, translate, currentLocale } from '$lib/i18n';

  let { children } = $props();
  let moreOpen = $state(false);
  let isDark = $state(false);
  let unreadNotifications = $state(0);
  let navLoading = $state(false);

  // Rail flotante: abierto por defecto; solo el botón lo expande/contrae
  let railOpen = $state(true);
  let railExpanded = $derived(railOpen);

  beforeNavigate(() => { navLoading = true; });
  afterNavigate(() => { navLoading = false; });

  interface Toast { id: number; message: string; href: string; tone: 'info' | 'alert'; }
  let toasts = $state<Toast[]>([]);
  let toastSeq = 0;

  const STORAGE_KEY = 'purrce_unread_notifications';

  const navGroups = [
    {
      label: 'nav.group.tracking',
      links: [
        { href: '/', label: 'nav.dashboard', icon: 'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6' },
        { href: '/products', label: 'nav.products', icon: 'M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4' },
        { href: '/tags', label: 'nav.tags', icon: 'M7 7h.01M7 3h5a2 2 0 011.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z' },
        { href: '/lists', label: 'nav.lists', icon: 'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2' },
      ],
    },
    {
      label: 'nav.group.alerts',
      links: [
        { href: '/notifications', label: 'nav.activity', icon: 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9' },
        { href: '/alerts', label: 'nav.alerts', icon: 'M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z' },
        { href: '/watches', label: 'nav.watches', icon: 'M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z' },
      ],
    },
    {
      label: 'nav.group.system',
      links: [
        { href: '/settings', label: 'nav.settings', icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z' },
      ],
    },
  ];

  const flatLinks = navGroups.flatMap(g => g.links);
  const mobilePrimaryHrefs = ['/', '/products', '/watches', '/notifications'];
  const mobilePrimary = mobilePrimaryHrefs
    .map(href => flatLinks.find(l => l.href === href))
    .filter((l): l is { href: string; label: string; icon: string } => !!l);
  const mobileMore = flatLinks.filter(l => !mobilePrimaryHrefs.includes(l.href));

  function pushToast(message: string, href: string, tone: 'info' | 'alert' = 'info') {
    const id = ++toastSeq;
    toasts = [...toasts, { id, message, href, tone }];
    setTimeout(() => { toasts = toasts.filter(t => t.id !== id); }, 5000);
  }

  const unsubscribeEvents = lastEvent.subscribe(ev => {
    if (!ev) return;
    if (ev.type === 'price_updated') {
      pushToast(translate(currentLocale(), 'toast.priceUpdated', { price: formatPrice(ev.new_price, ev.currency) }), '/products', 'info');
    }
    if (ev.type === 'alert_triggered') {
      if (page.url.pathname !== '/notifications') {
        unreadNotifications += 1;
        try { localStorage.setItem(STORAGE_KEY, String(unreadNotifications)); } catch {}
      }
      pushToast(translate(currentLocale(), 'toast.alertTriggered'), '/notifications', 'alert');
    }
  });

  onDestroy(() => { stopEventStream(); unsubscribeEvents(); });

  onMount(() => {
    initTheme();
    initLocale();
    startEventStream();
    isDark = document.documentElement.classList.contains('dark');
    try { unreadNotifications = parseInt(localStorage.getItem(STORAGE_KEY) || '0', 10) || 0; } catch {}
  });

  $effect(() => {
    page.url.pathname;
    moreOpen = false;
  });

  function handleToggleTheme() {
    toggleTheme();
    isDark = document.documentElement.classList.contains('dark');
  }

  function clearNotificationBadge() {
    unreadNotifications = 0;
    try { localStorage.removeItem(STORAGE_KEY); } catch {}
  }

  function isActive(href: string) {
    if (href === '/') return page.url.pathname === '/';
    return page.url.pathname.startsWith(href);
  }

  const brandIcon = 'M4 20V8.2L6.4 2l4.1 3.2h3L17.6 2 20 8.2V20a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z';
</script>

<div class="min-h-screen bg-[var(--bg)]" style="--rail-w: {railExpanded ? '15rem' : '4.75rem'}">
  {#if navLoading}
    <div class="nav-progress" aria-hidden="true"></div>
  {/if}

  <!-- Rail flotante desktop — M3 Expressive -->
  <aside
    class="m3-rail m3-surface fixed left-4 top-4 bottom-4 z-40 hidden lg:flex flex-col rounded-[28px] p-3 overflow-hidden {railExpanded ? '' : 'm3-collapsed'}"
    style="width: {railExpanded ? '15rem' : '4.75rem'}"
  >
    <!-- Marca -->
    <div class="flex items-center h-12 mb-1 shrink-0 {railExpanded ? 'px-1.5' : 'justify-center'}">
      <a href="/" class="flex items-center gap-2.5 min-w-0">
        <span class="w-9 h-9 shrink-0 rounded-full bg-[var(--accent)] text-[var(--accent-fg)] flex items-center justify-center">
          <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9" d={brandIcon} />
            <circle cx="9.5" cy="13" r="1.15" fill="currentColor" stroke="none" />
            <circle cx="14.5" cy="13" r="1.15" fill="currentColor" stroke="none" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9" d="M10.2 16.6c.5.5 1.1.8 1.8.8s1.3-.3 1.8-.8" />
          </svg>
        </span>
        <span class="m3-label grid" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
          <span class="overflow-hidden text-[15px] font-semibold tracking-tight whitespace-nowrap">Purrce</span>
        </span>
      </a>
    </div>

    <nav class="flex-1 overflow-y-auto overflow-x-hidden space-y-3">
      {#each navGroups as group}
        <div>
          <p class="m3-label grid px-3 pb-1.5" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
            <span class="overflow-hidden section-label whitespace-nowrap">{$t(group.label)}</span>
          </p>
          <div class="space-y-0.5">
            {#each group.links as link}
              {@const active = isActive(link.href)}
              <a
                href={link.href}
                aria-current={active ? 'page' : undefined}
                onclick={link.href === '/notifications' ? clearNotificationBadge : undefined}
                class="m3-item group relative flex items-center gap-3 h-11 rounded-full px-2.5
                  {active ? 'bg-[var(--accent-subtle)]' : 'hover:bg-[var(--bg-subtle)]'}"
              >
                <span
                  class="m3-icon w-8 h-8 shrink-0 rounded-full flex items-center justify-center
                    {active
                      ? 'bg-[var(--accent)] text-[var(--accent-fg)]'
                      : 'text-[var(--fg-secondary)] group-hover:text-[var(--fg)]'}"
                >
                  <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d={link.icon} />
                  </svg>
                </span>
                <span class="m3-label flex-1 min-w-0 grid" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
                  <span class="overflow-hidden whitespace-nowrap text-[13px] font-medium {active ? 'text-[var(--accent)]' : 'text-[var(--fg-secondary)] group-hover:text-[var(--fg)]'}">
                    {$t(link.label)}
                  </span>
                </span>
                {#if link.href === '/notifications' && unreadNotifications > 0}
                  <span class="m3-label grid" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
                    <span class="overflow-hidden">
                      <span class="inline-flex min-w-[16px] h-4 px-1 rounded-full bg-[var(--accent)] text-[var(--accent-fg)] text-[10px] font-bold items-center justify-center">
                        {unreadNotifications > 9 ? '9+' : unreadNotifications}
                      </span>
                    </span>
                  </span>
                {/if}
              </a>
            {/each}
          </div>
        </div>
      {/each}
    </nav>

    <!-- Acciones del sistema -->
    <div class="pt-2 mt-1 border-t border-[var(--border-subtle)] space-y-0.5 shrink-0">
      <button
        onclick={handleToggleTheme}
        class="m3-item group w-full flex items-center gap-3 h-11 rounded-full px-2.5 text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]"
      >
        <span class="m3-icon w-8 h-8 shrink-0 rounded-full flex items-center justify-center group-hover:text-[var(--fg)]">
          <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            {#if isDark}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            {:else}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            {/if}
          </svg>
        </span>
        <span class="m3-label flex-1 min-w-0 grid" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
          <span class="overflow-hidden text-left whitespace-nowrap text-[13px] font-medium">{isDark ? $t('theme.light') : $t('theme.dark')}</span>
        </span>
      </button>

      <button
        onclick={() => railOpen = !railOpen}
        aria-pressed={railOpen}
        class="m3-item group w-full flex items-center gap-3 h-11 rounded-full px-2.5 text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]"
      >
        <span class="m3-icon w-8 h-8 shrink-0 rounded-full flex items-center justify-center group-hover:text-[var(--fg)]">
          <svg class="w-[18px] h-[18px] transition-transform duration-300 {railOpen ? '' : 'rotate-180'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M11 19l-7-7 7-7M18 19l-7-7 7-7" />
          </svg>
        </span>
        <span class="m3-label flex-1 min-w-0 grid" style="grid-template-columns: {railExpanded ? '1fr' : '0fr'}">
          <span class="overflow-hidden text-left whitespace-nowrap text-[13px] font-medium">{railOpen ? $t('rail.collapse') : $t('rail.expand')}</span>
        </span>
      </button>
    </div>
  </aside>

  <!-- Toolbar flotante móvil — M3 Expressive -->
  <nav class="m3-toolbar lg:hidden fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-0.5 p-1.5 rounded-full">
    {#each mobilePrimary as link}
      {@const active = isActive(link.href)}
      <a
        href={link.href}
        aria-current={active ? 'page' : undefined}
        aria-label={$t(link.label)}
        onclick={link.href === '/notifications' ? clearNotificationBadge : undefined}
        class="relative h-12 rounded-full flex items-center justify-center px-3 transition-colors
          {active ? 'bg-[var(--accent-subtle)] text-[var(--accent)]' : 'text-[var(--fg-secondary)]'}"
      >
        <span class="relative shrink-0">
          <svg class="w-[22px] h-[22px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d={link.icon} />
          </svg>
          {#if link.href === '/notifications' && unreadNotifications > 0}
            <span class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-[var(--accent)] ring-2 ring-[var(--bg-elevated)]"></span>
          {/if}
        </span>
        <span class="m3-label grid" style="grid-template-columns: {active ? '1fr' : '0fr'}">
          <span class="overflow-hidden whitespace-nowrap">
            <span class="pl-2 pr-0.5 text-[13px] font-semibold">{$t(link.label)}</span>
          </span>
        </span>
      </a>
    {/each}

    <button
      onclick={() => moreOpen = !moreOpen}
      aria-label={$t("mobile.more")}
      aria-expanded={moreOpen}
      class="m3-item w-12 h-12 rounded-full flex items-center justify-center
        {moreOpen ? 'bg-[var(--accent-subtle)] text-[var(--accent)]' : 'text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]'}"
    >
      <svg class="w-[22px] h-[22px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M5 12h.01M12 12h.01M19 12h.01" />
      </svg>
    </button>
  </nav>

  <!-- Hoja "Más" móvil -->
  {#if moreOpen}
    <div class="lg:hidden fixed inset-0 z-40" transition:fade={{ duration: 150 }}>
      <div class="absolute inset-0 bg-black/40" onclick={() => moreOpen = false}></div>
    </div>
    <div
      class="lg:hidden fixed bottom-24 left-1/2 -translate-x-1/2 z-[70] w-[min(92vw,420px)] m3-surface rounded-[28px] p-3"
      transition:fly={{ y: 16, duration: 220 }}
    >
      <p class="section-label px-3 pb-2">{$t("mobile.more")}</p>
      <div class="space-y-0.5">
        {#each mobileMore as link}
          {@const active = isActive(link.href)}
          <a
            href={link.href}
            aria-current={active ? 'page' : undefined}
            onclick={link.href === '/notifications' ? clearNotificationBadge : undefined}
            class="m3-item flex items-center gap-3 h-12 rounded-full px-3
              {active ? 'bg-[var(--accent-subtle)] text-[var(--accent)]' : 'text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]'}"
          >
            <span class="w-8 h-8 shrink-0 rounded-full flex items-center justify-center {active ? 'bg-[var(--accent)] text-[var(--accent-fg)]' : ''}">
              <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d={link.icon} />
              </svg>
            </span>
            <span class="flex-1 text-[14px] font-medium">{$t(link.label)}</span>
            {#if link.href === '/notifications' && unreadNotifications > 0}
              <span class="min-w-[18px] h-[18px] px-1 rounded-full bg-[var(--accent)] text-[var(--accent-fg)] text-[10px] font-bold flex items-center justify-center">
                {unreadNotifications > 9 ? '9+' : unreadNotifications}
              </span>
            {/if}
          </a>
        {/each}

        <button
          onclick={handleToggleTheme}
          class="m3-item w-full flex items-center gap-3 h-12 rounded-full px-3 text-[var(--fg-secondary)] hover:bg-[var(--bg-subtle)] hover:text-[var(--fg)]"
        >
          <span class="w-8 h-8 shrink-0 rounded-full flex items-center justify-center">
            <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              {#if isDark}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
              {:else}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
              {/if}
            </svg>
          </span>
          <span class="flex-1 text-left text-[14px] font-medium">{isDark ? $t('theme.light') : $t('theme.dark')}</span>
        </button>
      </div>
    </div>
  {/if}

  <main class="app-main">
    <div class="max-w-[1200px] mx-auto px-5 sm:px-8 py-7 sm:py-9 pb-28 lg:pb-9">
      {#key page.url.pathname}
        <div in:fade={{ duration: 200, delay: 30 }}>
          {@render children()}
        </div>
      {/key}
    </div>
  </main>

  <!-- Toasts en vivo (SSE) -->
  <div class="fixed bottom-4 right-4 z-[60] flex flex-col items-end gap-2 pointer-events-none" aria-live="polite">
    {#each toasts as toast (toast.id)}
      <a
        href={toast.href}
        class="animate-toast pointer-events-auto card px-4 py-3 shadow-[var(--shadow-lg)] flex items-center gap-3 max-w-[320px] hover:border-[var(--border-strong)] transition-colors"
        out:fly={{ y: 10, duration: 180 }}
      >
        <span class="w-2 h-2 rounded-full flex-shrink-0 {toast.tone === 'alert' ? 'bg-[var(--accent)]' : 'bg-[var(--success)]'}"></span>
        <span class="text-[13px] font-medium">{toast.message}</span>
      </a>
    {/each}
  </div>
</div>
