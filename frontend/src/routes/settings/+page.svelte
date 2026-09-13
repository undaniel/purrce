<script lang="ts">
  import { onMount } from 'svelte';
  import { theme, setTheme } from '$lib/stores/theme';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { t, translate, currentLocale } from '$lib/i18n';

  let currentTheme = $state<'light' | 'dark' | 'system'>('system');
  theme.subscribe(v => { currentTheme = v; });

  const themeOptions = [
    { value: 'light', label: 'settings.theme.light', icon: 'M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z' },
    { value: 'dark', label: 'settings.theme.dark', icon: 'M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z' },
    { value: 'system', label: 'settings.theme.system', icon: 'M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z' },
  ];

  interface NotifConfig {
    telegram_enabled: boolean; telegram_bot_token: string; telegram_chat_id: string;
    email_enabled: boolean; email_smtp_host: string; email_smtp_port: number; email_from: string; email_to: string;
    webhook_enabled: boolean; webhook_url: string;
    ntfy_enabled: boolean; ntfy_url: string; ntfy_topic: string; ntfy_token: string;
  }
  let config = $state<NotifConfig>({
    telegram_enabled: false, telegram_bot_token: '', telegram_chat_id: '',
    email_enabled: false, email_smtp_host: '', email_smtp_port: 587, email_from: '', email_to: '',
    webhook_enabled: false, webhook_url: '',
    ntfy_enabled: false, ntfy_url: 'https://ntfy.sh', ntfy_topic: '', ntfy_token: '',
  });
  let loadingConfig = $state(true);
  let saving = $state(false);
  let testing = $state<string | null>(null);
  let toast = $state<{ message: string; type: 'success' | 'error' } | null>(null);

  onMount(async () => {
    try {
      const r = await fetch('/api/notifications/config');
      if (r.ok) config = await r.json();
    } catch {}
    loadingConfig = false;
  });

  function showToast(message: string, type: 'success' | 'error') {
    toast = { message, type };
    setTimeout(() => toast = null, 3000);
  }

  async function saveConfig() {
    saving = true;
    try {
      const r = await fetch('/api/notifications/config', {
        method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(config)
      });
      showToast(r.ok ? translate(currentLocale(), 'settings.saved') : translate(currentLocale(), 'settings.saveError'), r.ok ? 'success' : 'error');
    } catch { showToast(translate(currentLocale(), 'settings.connectionError'), 'error'); }
    saving = false;
  }

  async function testNotification(type: 'telegram' | 'ntfy') {
    testing = type;
    try {
      await fetch('/api/notifications/config', {
        method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(config)
      });
      const r = await fetch(`/api/notifications/test/${type}`, { method: 'POST' });
      const d = await r.json();
      showToast(r.ok ? translate(currentLocale(), 'settings.testSent') : (d.error?.message || translate(currentLocale(), 'settings.testError')), r.ok ? 'success' : 'error');
    } catch { showToast(translate(currentLocale(), 'settings.connectionError'), 'error'); }
    testing = null;
  }
</script>

<svelte:head>
  <title>{$t('settings.pageTitle')}</title>
</svelte:head>

{#if toast}
  <div class="fixed top-4 right-4 z-50">
    <div class="px-4 py-2.5 rounded-[var(--radius-md)] text-[13px] font-medium shadow-[var(--shadow-lg)] animate-in {toast.type === 'success' ? 'bg-[var(--success)] text-white' : 'bg-[var(--danger)] text-white'}">
      {toast.message}
    </div>
  </div>
{/if}

<div class="space-y-5">
  <div>
    <h1 class="page-title">{$t('nav.settings')}</h1>
    <p class="text-[13px] text-[var(--fg-muted)] mt-0.5">{$t('settings.subtitle')}</p>
  </div>

  <!-- Apariencia -->
  <div class="card p-4 sm:p-5">
    <h2 class="section-title mb-4">{$t('settings.appearance')}</h2>
    <div class="grid grid-cols-3 gap-2 sm:gap-3">
      {#each themeOptions as option}
        <button
          onclick={() => setTheme(option.value as 'light' | 'dark' | 'system')}
          class="flex flex-col items-center gap-2 p-3 sm:p-4 rounded-[var(--radius-md)] border transition-colors
            {currentTheme === option.value
              ? 'bg-[var(--accent-subtle)] border-[var(--accent)]/40 text-[var(--accent)]'
              : 'bg-[var(--bg-subtle)] border-transparent text-[var(--fg-muted)] hover:text-[var(--fg)]'}"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={option.icon} />
          </svg>
          <span class="text-[12px] font-medium">{$t(option.label)}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Canales de notificación -->
  <div class="card overflow-hidden">
    <div class="flex items-center justify-between p-4 sm:p-5 border-b border-[var(--border)]">
      <div>
        <h2 class="section-title">{$t('settings.channels')}</h2>
        <p class="text-[12px] text-[var(--fg-muted)] mt-0.5">{$t('settings.channelsHint')}</p>
      </div>
      <button onclick={saveConfig} disabled={saving || loadingConfig} class="btn btn-primary btn-sm">
        {saving ? $t('settings.saving') : $t('settings.save')}
      </button>
    </div>

    {#if loadingConfig}
      <div class="p-4 space-y-3">
        {#each [1, 2, 3, 4] as _}
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <Skeleton width="36px" height="36px" rounded="var(--radius-md)" />
              <div class="space-y-1.5">
                <Skeleton width="80px" height="13px" />
                <Skeleton width="130px" height="10px" />
              </div>
            </div>
            <Skeleton width="40px" height="22px" rounded="11px" />
          </div>
        {/each}
      </div>
    {:else}
      <div class="divide-y divide-[var(--border)]">

        <!-- Telegram -->
        <div>
          <button
            onclick={() => config.telegram_enabled = !config.telegram_enabled}
            class="w-full p-4 flex items-center justify-between hover:bg-[var(--bg-subtle)] transition-colors"
          >
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-[var(--radius-md)] flex items-center justify-center" style="background-color: {config.telegram_enabled ? '#229ED9' : 'rgba(34,158,217,0.12)'}">
                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor" style="color: {config.telegram_enabled ? 'white' : '#229ED9'}">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm4.64 6.8c-.15 1.58-.8 5.42-1.13 7.19-.14.75-.42 1-.68 1.03-.58.05-1.02-.38-1.58-.75-.88-.58-1.38-.94-2.23-1.5-.99-.65-.35-1.01.22-1.59.15-.15 2.71-2.48 2.76-2.69a.2.2 0 00-.05-.18c-.06-.05-.14-.03-.21-.02-.09.02-1.49.95-4.22 2.79-.4.27-.76.41-1.08.4-.36-.01-1.04-.2-1.55-.37-.63-.2-1.12-.31-1.08-.66.02-.18.27-.36.74-.55 2.92-1.27 4.86-2.11 5.83-2.51 2.78-1.16 3.35-1.36 3.73-1.36.08 0 .27.02.39.12.1.08.13.19.14.27-.01.06.01.24 0 .38z"/>
                </svg>
              </div>
              <div class="text-left">
                <p class="text-[13px] font-medium">Telegram</p>
                <p class="text-[11px] text-[var(--fg-muted)]">{$t('settings.telegramDesc')}</p>
              </div>
            </div>
            <div class="w-10 h-[22px] rounded-full transition-colors relative flex-shrink-0 {config.telegram_enabled ? 'bg-[#229ED9]' : 'bg-[var(--border)]'}">
              <div class="absolute top-[2px] w-[18px] h-[18px] rounded-full bg-white shadow-sm transition-transform {config.telegram_enabled ? 'translate-x-[20px]' : 'translate-x-[2px]'}"></div>
            </div>
          </button>
          {#if config.telegram_enabled}
            <div class="px-4 pb-4 space-y-3">
              <div>
                <label for="bot_token" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.botToken')}</label>
                <input id="bot_token" type="password" bind:value={config.telegram_bot_token} placeholder="123456789:ABCdefGHI..." class="input" />
                <p class="text-[11px] text-[var(--fg-muted)] mt-1">{$t('settings.botTokenHint')} <a href="https://t.me/BotFather" target="_blank" class="text-[#229ED9] hover:underline">@BotFather</a></p>
              </div>
              <div>
                <label for="chat_id" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">Chat ID</label>
                <input id="chat_id" type="text" bind:value={config.telegram_chat_id} placeholder="-1001234567890" class="input" />
              </div>
              <button onclick={() => testNotification('telegram')} disabled={testing !== null || !config.telegram_bot_token || !config.telegram_chat_id} class="btn btn-secondary btn-sm w-full">
                {testing === 'telegram' ? $t('settings.sending') : $t('settings.testConnection')}
              </button>
            </div>
          {/if}
        </div>

        <!-- ntfy -->
        <div>
          <button onclick={() => config.ntfy_enabled = !config.ntfy_enabled} class="w-full p-4 flex items-center justify-between hover:bg-[var(--bg-subtle)] transition-colors">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-[var(--radius-md)] flex items-center justify-center" style="background-color: {config.ntfy_enabled ? '#317EAD' : 'rgba(49,126,173,0.12)'}">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: {config.ntfy_enabled ? 'white' : '#317EAD'}" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
                </svg>
              </div>
              <div class="text-left">
                <p class="text-[13px] font-medium">ntfy</p>
                <p class="text-[11px] text-[var(--fg-muted)]">{$t('settings.ntfyDesc')}</p>
              </div>
            </div>
            <div class="w-10 h-[22px] rounded-full transition-colors relative flex-shrink-0 {config.ntfy_enabled ? 'bg-[#317EAD]' : 'bg-[var(--border)]'}">
              <div class="absolute top-[2px] w-[18px] h-[18px] rounded-full bg-white shadow-sm transition-transform {config.ntfy_enabled ? 'translate-x-[20px]' : 'translate-x-[2px]'}"></div>
            </div>
          </button>
          {#if config.ntfy_enabled}
            <div class="px-4 pb-4 space-y-3">
              <div>
                <label for="ntfy_url" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.server')}</label>
                <input id="ntfy_url" type="url" bind:value={config.ntfy_url} placeholder="https://ntfy.sh" class="input" />
              </div>
              <div>
                <label for="ntfy_topic" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.topic')}</label>
                <input id="ntfy_topic" type="text" bind:value={config.ntfy_topic} placeholder={$t('settings.topicPlaceholder')} class="input" />
              </div>
              <div>
                <label for="ntfy_token" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">Token <span class="font-normal">{$t('settings.optional')}</span></label>
                <input id="ntfy_token" type="password" bind:value={config.ntfy_token} placeholder="tk_..." class="input" />
              </div>
              <button onclick={() => testNotification('ntfy')} disabled={testing !== null || !config.ntfy_topic} class="btn btn-secondary btn-sm w-full">
                {testing === 'ntfy' ? $t('settings.sending') : $t('settings.testConnection')}
              </button>
            </div>
          {/if}
        </div>

        <!-- Email -->
        <div>
          <button onclick={() => config.email_enabled = !config.email_enabled} class="w-full p-4 flex items-center justify-between hover:bg-[var(--bg-subtle)] transition-colors">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-[var(--radius-md)] flex items-center justify-center" style="background-color: {config.email_enabled ? 'var(--success)' : 'var(--success-subtle)'}">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: {config.email_enabled ? 'white' : 'var(--success)'}" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                </svg>
              </div>
              <div class="text-left">
                <p class="text-[13px] font-medium">Email</p>
                <p class="text-[11px] text-[var(--fg-muted)]">{$t('settings.emailDesc')}</p>
              </div>
            </div>
            <div class="w-10 h-[22px] rounded-full transition-colors relative flex-shrink-0 {config.email_enabled ? 'bg-[var(--success)]' : 'bg-[var(--border)]'}">
              <div class="absolute top-[2px] w-[18px] h-[18px] rounded-full bg-white shadow-sm transition-transform {config.email_enabled ? 'translate-x-[20px]' : 'translate-x-[2px]'}"></div>
            </div>
          </button>
          {#if config.email_enabled}
            <div class="px-4 pb-4 space-y-3">
              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label for="smtp_host" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.smtpServer')}</label>
                  <input id="smtp_host" type="text" bind:value={config.email_smtp_host} placeholder="smtp.gmail.com" class="input" />
                </div>
                <div>
                  <label for="smtp_port" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.port')}</label>
                  <input id="smtp_port" type="number" bind:value={config.email_smtp_port} placeholder="587" class="input num" />
                </div>
              </div>
              <div>
                <label for="email_from" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.sender')}</label>
                <input id="email_from" type="email" bind:value={config.email_from} placeholder={$t('settings.senderPlaceholder')} class="input" />
              </div>
              <div>
                <label for="email_to" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.recipient')}</label>
                <input id="email_to" type="email" bind:value={config.email_to} placeholder={$t('settings.recipientPlaceholder')} class="input" />
              </div>
            </div>
          {/if}
        </div>

        <!-- Webhook -->
        <div>
          <button onclick={() => config.webhook_enabled = !config.webhook_enabled} class="w-full p-4 flex items-center justify-between hover:bg-[var(--bg-subtle)] transition-colors">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-[var(--radius-md)] flex items-center justify-center" style="background-color: {config.webhook_enabled ? '#7c3aed' : 'rgba(124,58,237,0.12)'}">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: {config.webhook_enabled ? 'white' : '#7c3aed'}" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
                </svg>
              </div>
              <div class="text-left">
                <p class="text-[13px] font-medium">Webhook</p>
                <p class="text-[11px] text-[var(--fg-muted)]">Slack, Discord, n8n, Zapier…</p>
              </div>
            </div>
            <div class="w-10 h-[22px] rounded-full transition-colors relative flex-shrink-0 {config.webhook_enabled ? 'bg-[#7c3aed]' : 'bg-[var(--border)]'}">
              <div class="absolute top-[2px] w-[18px] h-[18px] rounded-full bg-white shadow-sm transition-transform {config.webhook_enabled ? 'translate-x-[20px]' : 'translate-x-[2px]'}"></div>
            </div>
          </button>
          {#if config.webhook_enabled}
            <div class="px-4 pb-4">
              <label for="webhook_url" class="block text-[11px] font-medium text-[var(--fg-muted)] mb-1">{$t('settings.webhookUrl')}</label>
              <input id="webhook_url" type="url" bind:value={config.webhook_url} placeholder="https://hooks.slack.com/services/..." class="input" />
            </div>
          {/if}
        </div>

      </div>
    {/if}
  </div>

  <!-- Sobre Purrce -->
  <div class="card p-4 sm:p-5">
    <h2 class="section-title mb-2">{$t('settings.about')}</h2>
    <p class="text-[13px] text-[var(--fg-muted)] leading-relaxed">
      {$t('settings.aboutText')}
    </p>
    <p class="num text-[11px] text-[var(--fg-muted)] mt-3">{$t('settings.version', { version: '1.0.1' })}</p>
  </div>
</div>
