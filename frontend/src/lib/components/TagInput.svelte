<script lang="ts">
  import type { Tag } from '$lib/types';
  import { api } from '$lib/api/client';
  import { t } from '$lib/i18n';

  interface Props {
    tags: Tag[];
    onTagsChange: (tags: Tag[]) => void;
  }

  let { tags, onTagsChange }: Props = $props();
  
  let inputValue = $state('');
  let suggestions: Tag[] = $state([]);
  let showSuggestions = $state(false);
  let allTags: Tag[] = $state([]);
  let loaded = $state(false);
  let addingTag = $state(false);

  async function loadAllTags() {
    if (loaded) return;
    try {
      allTags = await api.tags.list();
      loaded = true;
    } catch (e) {
      console.error('Error loading tags:', e);
    }
  }

  function updateSuggestions() {
    if (!inputValue.trim()) {
      suggestions = [];
      showSuggestions = false;
      return;
    }
    const existingNames = new Set(tags.map(t => t.name.toLowerCase()));
    suggestions = allTags
      .filter(t => 
        t.name.toLowerCase().includes(inputValue.toLowerCase()) && 
        !existingNames.has(t.name.toLowerCase())
      )
      .slice(0, 5);
    showSuggestions = suggestions.length > 0 || inputValue.trim().length > 0;
  }

  async function addTag(name: string) {
    const trimmed = name.trim();
    if (!trimmed || addingTag) return;
    
    const existingNames = new Set(tags.map(t => t.name.toLowerCase()));
    if (existingNames.has(trimmed.toLowerCase())) return;

    addingTag = true;
    try {
      let tag = allTags.find(t => t.name.toLowerCase() === trimmed.toLowerCase());
      if (!tag) {
        tag = await api.tags.create(trimmed);
        allTags = [...allTags, tag];
      }
      onTagsChange([...tags, tag]);
      inputValue = '';
      showSuggestions = false;
    } catch (e) {
      console.error('Error adding tag:', e);
    } finally {
      addingTag = false;
    }
  }

  function removeTag(tagId: string) {
    onTagsChange(tags.filter(t => t.id !== tagId));
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (suggestions.length > 0) {
        addTag(suggestions[0].name);
      } else if (inputValue.trim()) {
        addTag(inputValue);
      }
    } else if (e.key === 'Escape') {
      showSuggestions = false;
    }
  }

  function handleInput() {
    updateSuggestions();
  }

  function handleFocus() {
    loadAllTags();
    updateSuggestions();
  }

  function handleBlur() {
    setTimeout(() => {
      showSuggestions = false;
    }, 200);
  }
</script>

<div class="space-y-2">
  {#if tags.length > 0}
    <div class="flex flex-wrap gap-1.5">
      {#each tags as tag}
        <span 
          class="inline-flex items-center gap-1 text-[12px] px-2 py-0.5 rounded font-medium"
          style="background-color: {tag.color}12; color: {tag.color}"
        >
          {tag.name}
          <button
            type="button"
            onclick={() => removeTag(tag.id)}
            disabled={addingTag}
            class="ml-0.5 hover:opacity-70 disabled:opacity-50 transition-opacity"
            aria-label={$t('taginput.remove', { name: tag.name })}
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </span>
      {/each}
    </div>
  {/if}

  <div class="relative">
    <div class="relative">
      <input
        type="text"
        bind:value={inputValue}
        oninput={handleInput}
        onkeydown={handleKeydown}
        onfocus={handleFocus}
        onblur={handleBlur}
        disabled={addingTag}
        placeholder={$t('taginput.placeholder')}
        class="input disabled:opacity-50"
      />
      {#if addingTag}
        <div class="absolute right-3 top-1/2 -translate-y-1/2">
          <svg class="w-3.5 h-3.5 animate-spin text-[var(--accent)]" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        </div>
      {/if}
    </div>
    
    {#if showSuggestions && !addingTag}
      <div class="absolute z-50 w-full mt-1 bg-[var(--bg-elevated)] border border-[var(--border)] rounded-[var(--radius-md)] shadow-[var(--shadow-lg)] overflow-hidden">
        {#each suggestions as suggestion}
          <button
            type="button"
            onmousedown={(e) => { e.preventDefault(); addTag(suggestion.name); }}
            class="w-full px-3.5 py-2.5 text-left text-[13px] hover:bg-[var(--bg-subtle)] flex items-center gap-2 transition-colors"
          >
            <span 
              class="w-2 h-2 rounded-full flex-shrink-0" 
              style="background-color: {suggestion.color}"
            ></span>
            {suggestion.name}
          </button>
        {/each}
        {#if inputValue.trim() && !allTags.some(t => t.name.toLowerCase() === inputValue.trim().toLowerCase())}
          <button
            type="button"
            onmousedown={(e) => { e.preventDefault(); addTag(inputValue); }}
            class="w-full px-3.5 py-2.5 text-left text-[13px] hover:bg-[var(--bg-subtle)] flex items-center gap-2 border-t border-[var(--border)] transition-colors"
          >
            <svg class="w-3.5 h-3.5 text-[var(--fg-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span>{$t('taginput.create')} "<span class="font-medium">{inputValue.trim()}</span>"</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>
