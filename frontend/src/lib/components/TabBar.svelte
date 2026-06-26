<script lang="ts">
  import { store, disconnect } from '$lib/stores/connections.svelte'
  import SettingsPanel from './SettingsPanel.svelte'
  import { cn } from '$lib/utils'

  let settingsOpen = $state(false)

  const STATUS_COLOR: Record<string, string> = {
    connected:    'bg-green-400',
    connecting:   'bg-yellow-400 animate-pulse',
    error:        'bg-red-400',
    disconnected: 'bg-zinc-600',
    idle:         'bg-zinc-600',
  }
</script>

<div class="flex h-9 items-stretch border-b border-border bg-muted/20 overflow-x-auto shrink-0">
  {#each store.connections as conn (conn.id)}
    <button
      class={cn(
        'group flex items-center gap-2 border-r border-border px-4 text-xs font-medium transition-colors whitespace-nowrap',
        conn.id === store.activeId
          ? 'bg-background text-foreground'
          : 'text-muted-foreground hover:text-foreground hover:bg-muted/50',
      )}
      onclick={() => { store.activeId = conn.id; store.newConnOpen = false }}
    >
      <span class={cn('size-1.5 shrink-0 rounded-full', STATUS_COLOR[conn.status])}></span>
      {conn.label}
      <span
        role="button"
        tabindex="0"
        class="ml-0.5 flex size-4 items-center justify-center rounded text-muted-foreground opacity-0 transition-opacity hover:bg-muted hover:text-foreground group-hover:opacity-100"
        onclick={(e) => { e.stopPropagation(); disconnect(conn.id) }}
        onkeydown={(e) => e.key === 'Enter' && (e.stopPropagation(), disconnect(conn.id))}
      >×</span>
    </button>
  {/each}

  <!-- Pseudo-tab shown while the new-connection form is open (only when other connections exist) -->
  {#if store.newConnOpen && store.connections.length > 0}
    <div
      class="flex items-center gap-2 border-r border-border bg-background px-4 text-xs font-medium text-foreground"
    >
      <span class="size-1.5 shrink-0 rounded-full bg-zinc-600"></span>
      New connection
      <span
        role="button"
        tabindex="0"
        class="ml-0.5 flex size-4 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
        title="Cancel"
        onclick={() => {
          store.newConnOpen = false
          store.activeId = store.connections[store.connections.length - 1].id
        }}
        onkeydown={(e) => e.key === 'Enter' && (() => {
          store.newConnOpen = false
          store.activeId = store.connections[store.connections.length - 1].id
        })()}
      >×</span>
    </div>
  {/if}

  <button
    class={cn(
      'flex items-center px-3 text-lg leading-none transition-colors',
      store.newConnOpen || store.connections.length === 0
        ? 'bg-muted/50 text-foreground'
        : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground',
    )}
    title="New connection"
    onclick={() => store.openNew()}
  >+</button>

  <div class="ml-auto flex items-center px-2">
    <button
      class={cn(
        'flex size-7 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground',
        settingsOpen && 'bg-muted/50 text-foreground'
      )}
      title="Settings"
      onclick={() => settingsOpen = !settingsOpen}
    >
      <!-- sliders / settings icon -->
      <svg class="size-4" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
        <line x1="2" y1="4" x2="14" y2="4"/>
        <line x1="2" y1="8" x2="14" y2="8"/>
        <line x1="2" y1="12" x2="14" y2="12"/>
        <circle cx="5"  cy="4"  r="1.5" fill="currentColor" stroke="none"/>
        <circle cx="10" cy="8"  r="1.5" fill="currentColor" stroke="none"/>
        <circle cx="7"  cy="12" r="1.5" fill="currentColor" stroke="none"/>
      </svg>
    </button>
  </div>
</div>

<SettingsPanel bind:open={settingsOpen} />
