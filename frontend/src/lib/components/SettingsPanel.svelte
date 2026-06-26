<script lang="ts">
  import { fly } from 'svelte/transition'
  import { themeStore, setTheme } from '$lib/stores/theme.svelte'
  import type { ThemePreference } from '$lib/stores/theme.svelte'
  import { cn } from '$lib/utils'

  let { open = $bindable(false) }: { open: boolean } = $props()

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
    </div>
  </div>
{/if}
