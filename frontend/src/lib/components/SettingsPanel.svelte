<script lang="ts">
  import { fly } from 'svelte/transition'
  import { themeStore, setTheme } from '$lib/stores/theme.svelte'
  import type { ThemePreference } from '$lib/stores/theme.svelte'
  import { profileStore } from '$lib/stores/profiles.svelte'
  import type { Profile } from '$lib/stores/profiles.svelte'
  import { cn } from '$lib/utils'

  let { open = $bindable(false) }: { open: boolean } = $props()

  let importError = $state<string | null>(null)

  function exportProfiles() {
    const json = JSON.stringify(profileStore.profiles, null, 2)
    const blob = new Blob([json], { type: 'application/json' })
    const url  = URL.createObjectURL(blob)
    const a    = document.createElement('a')
    a.href     = url
    a.download = `conduit-profiles-${new Date().toISOString().slice(0, 10)}.json`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  function importProfiles() {
    importError = null
    const input = document.createElement('input')
    input.type  = 'file'
    input.accept = '.json,application/json'
    input.onchange = () => {
      const file = input.files?.[0]
      if (!file) return
      const reader = new FileReader()
      reader.onload = (e) => {
        try {
          const data = JSON.parse(e.target?.result as string) as Profile[]
          if (!Array.isArray(data)) throw new Error('invalid format')
          const existingIds = new Set(profileStore.profiles.map(p => p.id))
          let added = 0
          for (const p of data) {
            if (typeof p.id !== 'string' || typeof p.name !== 'string' || !p.config) continue
            if (existingIds.has(p.id)) continue
            profileStore.profiles.push(p)
            existingIds.add(p.id)
            added++
          }
          // persist
          localStorage.setItem('conduit-profiles', JSON.stringify(profileStore.profiles))
          if (added === 0) importError = 'No new profiles found (all already present).'
        } catch {
          importError = 'Invalid file — expected a Conduit profiles JSON.'
        }
      }
      reader.readAsText(file)
    }
    input.click()
  }

  const THEMES: { v: ThemePreference; label: string; icon: string }[] = [
    { v: 'light',  label: 'Light',  icon: '☀' },
    { v: 'system', label: 'System', icon: '⬤' },
    { v: 'dark',   label: 'Dark',   icon: '☾' },
  ]
</script>

{#if open}
  <div
    class="fixed inset-0 z-40"
    aria-hidden="true"
    onclick={() => open = false}
  ></div>

  <div
    class="fixed right-0 top-0 z-50 flex h-full w-72 flex-col border-l border-border bg-background shadow-xl"
    transition:fly={{ x: 288, duration: 180 }}
  >
    <div class="flex items-center justify-between border-b border-border px-4 py-3">
      <span class="text-sm font-semibold text-foreground">Settings</span>
      <button
        class="flex size-6 items-center justify-center rounded text-lg leading-none text-muted-foreground hover:bg-muted hover:text-foreground"
        onclick={() => open = false}
      >×</button>
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      <!-- Appearance -->
      <section class="mb-6">
        <h3 class="mb-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          Appearance
        </h3>
        <div class="flex gap-0.5 rounded border border-border p-0.5">
          {#each THEMES as t}
            <button
              class={cn(
                'flex flex-1 items-center justify-center gap-1.5 rounded px-2 py-1.5 text-xs font-medium transition-colors',
                themeStore.preference === t.v
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              )}
              onclick={() => setTheme(t.v)}
            >
              <span class="text-sm leading-none">{t.icon}</span>
              {t.label}
            </button>
          {/each}
        </div>
      </section>

      <!-- Profiles -->
      <section>
        <h3 class="mb-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          Profiles ({profileStore.profiles.length})
        </h3>

        <div class="flex gap-2">
          <button
            class="flex flex-1 items-center justify-center gap-1.5 rounded border border-border px-3 py-2 text-xs text-foreground transition-colors hover:bg-muted disabled:cursor-default disabled:opacity-40"
            disabled={profileStore.profiles.length === 0}
            onclick={exportProfiles}
          >
            <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M8 2v8M5 7l3 3 3-3" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M3 11v2h10v-2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Export
          </button>
          <button
            class="flex flex-1 items-center justify-center gap-1.5 rounded border border-border px-3 py-2 text-xs text-foreground transition-colors hover:bg-muted"
            onclick={importProfiles}
          >
            <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M8 10V2M5 5l3-3 3 3" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M3 11v2h10v-2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Import
          </button>
        </div>

        {#if importError}
          <p class="mt-2 rounded border border-amber-800/40 bg-amber-950/20 px-2.5 py-1.5 text-[11px] text-amber-400">
            {importError}
          </p>
        {/if}

        {#if profileStore.profiles.length > 0}
          <ul class="mt-3 space-y-1">
            {#each profileStore.profiles as p}
              <li class="flex items-center gap-2 rounded border border-border px-2.5 py-1.5 text-xs">
                <span class="flex-1 truncate text-foreground">{p.name}</span>
                <span class="shrink-0 text-[10px] text-muted-foreground">
                  {p.config.type === 'serial' ? p.config.config.port : p.config.type.toUpperCase()}
                </span>
                <button
                  class="shrink-0 text-muted-foreground/50 transition-colors hover:text-red-400"
                  onclick={() => profileStore.remove(p.id)}
                  aria-label="Delete profile"
                >✕</button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="mt-2 text-[11px] text-muted-foreground/60">No saved profiles.</p>
        {/if}
      </section>
    </div>
  </div>
{/if}
