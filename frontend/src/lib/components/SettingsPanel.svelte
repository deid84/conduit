<script lang="ts">
  import { fly } from 'svelte/transition'
  import { themeStore, setTheme } from '$lib/stores/theme.svelte'
  import type { ThemePreference } from '$lib/stores/theme.svelte'
  import { profileStore } from '$lib/stores/profiles.svelte'
  import { store } from '$lib/stores/connections.svelte'
  import { cn } from '$lib/utils'
  import AboutModal from './AboutModal.svelte'

  let { open = $bindable(false) }: { open: boolean } = $props()

  let importError = $state<string | null>(null)
  let aboutOpen = $state(false)

  let exportOpen     = $state(false)
  let exportSelected = $state<Set<string>>(new Set())
  let exportStatus   = $state<string | null>(null)

  function openExport() {
    exportSelected = new Set(profileStore.profiles.map(p => p.id))
    exportStatus   = null
    exportOpen     = true
  }

  function toggleExport(id: string) {
    const next = new Set(exportSelected)
    if (next.has(id)) next.delete(id); else next.add(id)
    exportSelected = next
  }

  function toggleAllExport() {
    exportSelected = exportSelected.size === profileStore.profiles.length
      ? new Set()
      : new Set(profileStore.profiles.map(p => p.id))
  }

  async function exportProfiles() {
    const chosen   = profileStore.profiles.filter(p => exportSelected.has(p.id))
    if (chosen.length === 0) return
    const json     = JSON.stringify(chosen, null, 2)
    const base     = chosen.length === 1
      ? chosen[0].name.replace(/[^\w.-]+/g, '_')
      : 'conduit-profiles'
    const filename = `${base}-${new Date().toISOString().slice(0, 10)}.json`

    try {
      if ('__TAURI_INTERNALS__' in window) {
        const { save }          = await import('@tauri-apps/plugin-dialog')
        const { writeTextFile } = await import('@tauri-apps/plugin-fs')
        const path = await save({
          filters: [{ name: 'JSON', extensions: ['json'] }],
          defaultPath: filename,
        })
        if (!path) return
        await writeTextFile(path, json)
      } else if ('showSaveFilePicker' in window) {
        const handle = await (window as any).showSaveFilePicker({
          suggestedName: filename,
          types: [{ description: 'JSON', accept: { 'application/json': ['.json'] } }],
        })
        const writable = await handle.createWritable()
        await writable.write(json)
        await writable.close()
      } else {
        const blob = new Blob([json], { type: 'application/json' })
        const url  = URL.createObjectURL(blob)
        const a    = document.createElement('a')
        a.href     = url
        a.download = filename
        document.body.appendChild(a)
        a.click()
        document.body.removeChild(a)
        setTimeout(() => URL.revokeObjectURL(url), 100)
      }
      exportOpen = false
    } catch (e) {
      // user cancelled the browser save picker
      if (e instanceof DOMException && e.name === 'AbortError') return
      exportStatus = `Export failed: ${e instanceof Error ? e.message : String(e)}`
    }
  }

  function importProfiles() {
    importError = null
    const input   = document.createElement('input')
    input.type    = 'file'
    input.accept  = '.json,application/json'
    input.onchange = () => {
      const file = input.files?.[0]
      if (!file) return
      const reader = new FileReader()
      reader.onload = (e) => {
        try {
          const data  = JSON.parse(e.target?.result as string)
          const added = profileStore.importJson(data)
          if (added === 0) importError = 'No new profiles found (all already present).'
        } catch {
          importError = 'Invalid file — expected a Conduit profiles JSON.'
        }
      }
      reader.readAsText(file)
    }
    input.click()
  }

  function updateFromActive(profileId: string) {
    const conn = store.active
    if (!conn || conn.sourceProfileId !== profileId) return
    profileStore.update(profileId, {
      config:      conn.conn,
      modbusMode:  conn.modbusMode,
      monitorRows: conn.monitorRows,
    })
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
            onclick={openExport}
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
              {@const isActive = store.active?.sourceProfileId === p.id}
              <li class={cn(
                'flex items-center gap-2 rounded border px-2.5 py-1.5 text-xs',
                isActive ? 'border-blue-600/40 bg-blue-950/10' : 'border-border'
              )}>
                <span class="flex-1 truncate text-foreground">{p.name}</span>
                <span class="shrink-0 text-[10px] text-muted-foreground">
                  {p.config.type === 'serial' ? p.config.config.port : p.config.type.toUpperCase()}
                </span>
                {#if isActive}
                  <button
                    class="shrink-0 text-[10px] text-blue-400 transition-colors hover:text-blue-300"
                    onclick={() => updateFromActive(p.id)}
                    title="Update profile with current connection settings"
                  >Update</button>
                {/if}
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

    <!-- About -->
    <div class="border-t border-border p-4">
      <button
        class="flex w-full items-center justify-center gap-2 rounded border border-border px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        onclick={() => aboutOpen = true}
      >
        <svg class="size-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="8" cy="8" r="6"/>
          <line x1="8" y1="7" x2="8" y2="11"/>
          <circle cx="8" cy="5" r="0.5" fill="currentColor" stroke="none"/>
        </svg>
        About Conduit
      </button>
    </div>
  </div>
{/if}

{#if exportOpen}
  <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50">
    <div class="w-80 max-w-[90vw] rounded border border-border bg-background p-4 shadow-xl">
      <div class="mb-3 flex items-center justify-between">
        <span class="text-sm font-semibold text-foreground">Export profiles</span>
        <button class="text-lg leading-none text-muted-foreground hover:text-foreground" onclick={() => exportOpen = false}>×</button>
      </div>

      <label class="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
        <input
          type="checkbox"
          checked={exportSelected.size === profileStore.profiles.length}
          onchange={toggleAllExport}
        />
        Select all ({exportSelected.size}/{profileStore.profiles.length})
      </label>

      <ul class="mb-3 max-h-60 space-y-1 overflow-y-auto">
        {#each profileStore.profiles as p}
          <li>
            <label class="flex cursor-pointer items-center gap-2 rounded border border-border px-2.5 py-1.5 text-xs">
              <input type="checkbox" checked={exportSelected.has(p.id)} onchange={() => toggleExport(p.id)} />
              <span class="flex-1 truncate text-foreground">{p.name}</span>
              <span class="shrink-0 text-[10px] text-muted-foreground">{p.monitorRows.length} rows</span>
            </label>
          </li>
        {/each}
      </ul>

      {#if exportStatus}
        <p class="mb-2 text-[11px] text-red-400">{exportStatus}</p>
      {/if}

      <div class="flex justify-end gap-2">
        <button
          class="rounded border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted"
          onclick={() => exportOpen = false}
        >Cancel</button>
        <button
          class="rounded border border-blue-600/50 bg-blue-950/30 px-3 py-1.5 text-xs text-blue-300 hover:bg-blue-950/50 disabled:opacity-40"
          disabled={exportSelected.size === 0}
          onclick={exportProfiles}
        >Choose location…</button>
      </div>
    </div>
  </div>
{/if}

<AboutModal bind:open={aboutOpen} />
