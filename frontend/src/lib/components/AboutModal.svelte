<script lang="ts">
  import { fade, scale } from 'svelte/transition'

  let { open = $bindable(false) }: { open: boolean } = $props()

  function close() { open = false }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close()
  }

  async function openUrl(url: string) {
    if ('__TAURI_INTERNALS__' in window) {
      const { openUrl } = await import('@tauri-apps/plugin-opener')
      await openUrl(url)
    } else {
      window.open(url, '_blank', 'noreferrer')
    }
  }
</script>

<svelte:window {onkeydown} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
    aria-modal="true"
    role="dialog"
    onclick={close}
    transition:fade={{ duration: 150 }}
  >
    <div
      class="relative w-80 rounded-lg border border-border bg-background p-6 shadow-xl"
      onclick={(e) => e.stopPropagation()}
      transition:scale={{ start: 0.95, duration: 150 }}
    >
      <!-- close -->
      <button
        class="absolute right-3 top-3 flex size-6 items-center justify-center rounded text-lg leading-none text-muted-foreground hover:bg-muted hover:text-foreground"
        onclick={close}
        aria-label="Close"
      >×</button>

      <!-- logo + name -->
      <div class="mb-4 flex flex-col items-center gap-2 text-center">
        <img src="/icon.png" alt="Conduit" class="size-12 rounded-xl" />
        <div>
          <h2 class="text-base font-bold text-foreground">Conduit</h2>
          <p class="text-xs text-muted-foreground">v0.3.0</p>
        </div>
      </div>

      <!-- links -->
      <div class="flex flex-col gap-2">
        <button
          class="flex items-center justify-center gap-2 rounded border border-border px-3 py-2 text-xs text-foreground transition-colors hover:bg-muted"
          onclick={() => openUrl('https://github.com/deid84/conduit')}
        >
          <svg class="size-3.5" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38
              0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13
              -.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66
              .07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15
              -.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27
              .68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12
              .51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48
              0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
          </svg>
          GitHub
        </button>

        <button
          class="flex items-center justify-center gap-2 rounded bg-[#FFDD00] px-3 py-2 text-xs font-medium text-black transition-opacity hover:opacity-90"
          onclick={() => openUrl('https://buymeacoffee.com/deid84')}
        >
          ☕ Buy me a coffee
        </button>
      </div>

      <!-- TODO: built with Tauri + Svelte logos -->

      <!-- license -->
      <p class="mt-3 text-center text-[10px] text-muted-foreground/60">
        MIT · Apache-2.0 — © 2026 deid84
      </p>
    </div>
  </div>
{/if}
