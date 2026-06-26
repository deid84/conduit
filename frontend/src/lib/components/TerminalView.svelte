<script lang="ts">
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import type { IDisposable, ITheme } from '@xterm/xterm'
  import type { Connection, LogEntry } from '$lib/stores/connections.svelte'
  import { store } from '$lib/stores/connections.svelte'
  import { sendData } from '$lib/api'
  import { themeStore, effectiveTheme } from '$lib/stores/theme.svelte'
  import '@xterm/xterm/css/xterm.css'

  // Catppuccin Latte (light)
  const THEME_LATTE: ITheme = {
    background:          '#eff1f5',
    foreground:          '#4c4f69',
    cursor:              '#dc8a78',
    cursorAccent:        '#eff1f5',
    selectionBackground: '#bcc0cc',
    black:               '#5c5f77',
    red:                 '#d20f39',
    green:               '#40a02b',
    yellow:              '#df8e1d',
    blue:                '#1e66f5',
    magenta:             '#ea76cb',
    cyan:                '#179299',
    white:               '#acb0be',
    brightBlack:         '#6c6f85',
    brightRed:           '#d20f39',
    brightGreen:         '#40a02b',
    brightYellow:        '#df8e1d',
    brightBlue:          '#1e66f5',
    brightMagenta:       '#ea76cb',
    brightCyan:          '#179299',
    brightWhite:         '#bcc0cc',
  }

  // Catppuccin Mocha (dark)
  const THEME_MOCHA: ITheme = {
    background:          '#1e1e2e',
    foreground:          '#cdd6f4',
    cursor:              '#f5e0dc',
    cursorAccent:        '#1e1e2e',
    selectionBackground: '#45475a',
    black:               '#45475a',
    red:                 '#f38ba8',
    green:               '#a6e3a1',
    yellow:              '#f9e2af',
    blue:                '#89b4fa',
    magenta:             '#f5c2e7',
    cyan:                '#89dceb',
    white:               '#bac2de',
    brightBlack:         '#585b70',
    brightRed:           '#f38ba8',
    brightGreen:         '#a6e3a1',
    brightYellow:        '#f9e2af',
    brightBlue:          '#89b4fa',
    brightMagenta:       '#f5c2e7',
    brightCyan:          '#89dceb',
    brightWhite:         '#a6adc8',
  }

  let { connection }: { connection: Connection } = $props()

  let container  = $state<HTMLDivElement | null>(null)
  let term       = $state<Terminal | null>(null)
  let written    = 0       // cursor into connection.log — not reactive
  let lastViewMode: 'ascii' | 'hex' = connection.viewMode

  // Context menu state
  let ctxVisible = $state(false)
  let ctxX       = $state(0)
  let ctxY       = $state(0)

  function showCtxMenu(e: MouseEvent) {
    e.preventDefault()
    // Clamp so the menu doesn't overflow the viewport
    ctxX = Math.min(e.clientX, window.innerWidth  - 160)
    ctxY = Math.min(e.clientY, window.innerHeight - 80)
    ctxVisible = true
  }

  function clearAndClose() {
    store.clearLog(connection.id)
    ctxVisible = false
  }

  // Close on any click outside the menu
  $effect(() => {
    if (!ctxVisible) return
    const close = () => { ctxVisible = false }
    window.addEventListener('pointerdown', close, { once: true })
    return () => window.removeEventListener('pointerdown', close)
  })

  function formatHex(raw: number[]): string {
    return raw.map(b => b.toString(16).padStart(2, '0').toUpperCase()).join(' ')
  }

  function writeEntry(t: Terminal, entry: LogEntry, viewMode: 'ascii' | 'hex') {
    if (entry.direction === 'tx') t.write('\x1b[36m')
    if (viewMode === 'hex') {
      t.write(formatHex(entry.raw) + '\r\n')
    } else {
      t.write(new Uint8Array(entry.raw))
    }
    if (entry.direction === 'tx') t.write('\x1b[0m')
  }

  // Effect 1: create/destroy terminal when container mounts.
  // Does NOT read connection.viewMode — keeping it out of this effect's
  // dependency set so that toggling ASCII/HEX does not destroy+recreate the terminal.
  $effect(() => {
    if (!container) return

    const t = new Terminal({
      cursorBlink:  true,
      cursorStyle:  'bar',
      disableStdin: true,
      fontFamily:   "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
      fontSize:     12,
      lineHeight:   1.4,
      convertEol:   true,
      scrollback:   10_000,
      theme:        effectiveTheme() === 'dark' ? THEME_MOCHA : THEME_LATTE,
    })

    const fitAddon   = new FitAddon()
    const linksAddon = new WebLinksAddon()
    t.loadAddon(fitAddon)
    t.loadAddon(linksAddon)
    t.open(container)
    fitAddon.fit()

    const ro = new ResizeObserver(() => fitAddon.fit())
    ro.observe(container)

    container.addEventListener('contextmenu', showCtxMenu)

    term = t

    return () => {
      ro.disconnect()
      container.removeEventListener('contextmenu', showCtxMenu)
      t.dispose()
      term    = null
      written = 0
    }
  })

  // Effect 2: write new log entries; clear+rewrite when view mode changes.
  $effect(() => {
    if (!term) return
    const viewMode = connection.viewMode   // tracked
    const len      = connection.log.length // tracked
    const t        = term

    if (viewMode !== lastViewMode) {
      t.clear(); written = 0; lastViewMode = viewMode
    } else if (len < written) {
      // log was cleared externally — wipe the terminal display
      t.clear(); written = 0
    }

    for (; written < len; written++) {
      writeEntry(t, connection.log[written], viewMode)
    }
  })

  // Effect 3: raw mode — forward keystrokes to device.
  $effect(() => {
    const mode = connection.terminalMode
    const t    = term
    if (!t) return

    let listener: IDisposable | null = null
    if (mode === 'raw') {
      t.options.disableStdin = false
      const id = connection.id
      listener = t.onData((data: string) => {
        void sendData(id, new TextEncoder().encode(data))
      })
    } else {
      t.options.disableStdin = true
    }

    return () => { listener?.dispose() }
  })

  // Effect 4: sync xterm palette when light/dark/system preference changes.
  const xtermTheme = $derived(effectiveTheme() === 'dark' ? THEME_MOCHA : THEME_LATTE)

  $effect(() => {
    const theme = xtermTheme
    const t     = term
    if (!t) return
    t.options.theme = theme
  })
</script>

<div bind:this={container} class="flex-1 overflow-hidden" style="background-color: {xtermTheme.background}"></div>

{#if ctxVisible}
  <!-- position:fixed escapes overflow:hidden; pointerdown stopPropagation prevents the
       window listener from firing immediately and closing the menu on the same event. -->
  <div
    class="fixed z-50 min-w-36 overflow-hidden rounded border border-border bg-background py-1 shadow-lg text-xs"
    style="left: {ctxX}px; top: {ctxY}px"
    onpointerdown={(e) => e.stopPropagation()}
  >
    <button
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-foreground hover:bg-muted"
      onclick={clearAndClose}
    >
      <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M2 4h12M6 4V2h4v2M5 4l1 10h4l1-10" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Clear terminal
    </button>
  </div>
{/if}
