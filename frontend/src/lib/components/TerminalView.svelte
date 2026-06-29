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

  // Brand light — Bone canvas
  const THEME_LATTE: ITheme = {
    background:          '#F4F1EB',
    foreground:          '#1C1813',
    cursor:              '#FBB040',
    cursorAccent:        '#1C1813',
    selectionBackground: '#CFC9BC',
    black:               '#3A3028',
    red:                 '#C0392B',
    green:               '#4A8A4A',
    yellow:              '#C47F00',
    blue:                '#2E6DA4',
    magenta:             '#8B5A8B',
    cyan:                '#2A7A82',
    white:               '#7A6E60',
    brightBlack:         '#5A4E42',
    brightRed:           '#E05050',
    brightGreen:         '#6AAA6A',
    brightYellow:        '#D89020',
    brightBlue:          '#4A8DC4',
    brightMagenta:       '#A878A8',
    brightCyan:          '#4A9AA0',
    brightWhite:         '#1C1813',
  }

  // Brand dark — Cinder surface
  const THEME_MOCHA: ITheme = {
    background:          '#252019',
    foreground:          '#F4F1EB',
    cursor:              '#FBB040',
    cursorAccent:        '#1C1813',
    selectionBackground: '#433A2C',
    black:               '#332B22',
    red:                 '#E05555',
    green:               '#7AB87A',
    yellow:              '#FBB040',
    blue:                '#6A9FD8',
    magenta:             '#C084C0',
    cyan:                '#60B8C0',
    white:               '#D4C8BA',
    brightBlack:         '#5A4E42',
    brightRed:           '#E87878',
    brightGreen:         '#98D098',
    brightYellow:        '#FCC870',
    brightBlue:          '#8BBDE0',
    brightMagenta:       '#D4A0D4',
    brightCyan:          '#88CCD4',
    brightWhite:         '#F4F1EB',
  }

  let { connection }: { connection: Connection } = $props()

  let container  = $state<HTMLDivElement | null>(null)
  let term       = $state<Terminal | null>(null)
  let written    = 0       // cursor into connection.log — not reactive
  let lastViewMode: 'ascii' | 'hex' = connection.viewMode

  // Context menu state
  let ctxVisible      = $state(false)
  let ctxX            = $state(0)
  let ctxY            = $state(0)
  let ctxHasSelection = $state(false)

  function showCtxMenu(e: MouseEvent) {
    e.preventDefault()
    ctxHasSelection = !!(term?.getSelection())
    // Clamp so the menu doesn't overflow the viewport
    ctxX = Math.min(e.clientX, window.innerWidth  - 168)
    ctxY = Math.min(e.clientY, window.innerHeight - 96)
    ctxVisible = true
  }

  async function copySelection() {
    const sel = term?.getSelection() ?? ''
    if (sel) await navigator.clipboard.writeText(sel).catch(() => {})
    ctxVisible = false
  }

  async function pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText()
      if (text) await sendData(connection.id, new TextEncoder().encode(text))
    } catch { /* clipboard permission denied */ }
    ctxVisible = false
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

    // Keyboard copy/paste — intercepted before xterm processes the event.
    t.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.type !== 'keydown') return true
      const ctrl = e.ctrlKey || e.metaKey

      // Copy: Ctrl/Cmd+C when text is selected (Ctrl+C without selection in raw mode
      // must fall through so xterm can send the 0x03 interrupt byte).
      if (ctrl && e.key.toLowerCase() === 'c') {
        const sel = t.getSelection()
        if (sel) {
          navigator.clipboard.writeText(sel).catch(() => {})
          return false   // eat the event; don't send 0x03
        }
        return connection.terminalMode === 'raw'  // let raw-mode Ctrl+C through
      }

      // Paste: Ctrl/Cmd+V — sends clipboard content to the device.
      if (ctrl && e.key.toLowerCase() === 'v') {
        navigator.clipboard.readText()
          .then(text => { if (text) sendData(connection.id, new TextEncoder().encode(text)) })
          .catch(() => {})
        return false
      }

      return true
    })

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
    class="fixed z-50 min-w-40 overflow-hidden rounded border border-border bg-background py-1 shadow-lg text-xs"
    style="left: {ctxX}px; top: {ctxY}px"
    onpointerdown={(e) => e.stopPropagation()}
  >
    <!-- Copy -->
    <button
      class="flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left transition-colors
             {ctxHasSelection ? 'text-foreground hover:bg-muted' : 'cursor-default text-muted-foreground/40'}"
      onclick={copySelection}
      disabled={!ctxHasSelection}
    >
      <span class="flex items-center gap-2">
        <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="5" y="5" width="8" height="9" rx="1"/>
          <path d="M3 11V3a1 1 0 0 1 1-1h8" stroke-linecap="round"/>
        </svg>
        Copy
      </span>
      <span class="text-[10px] text-muted-foreground/60">Ctrl+C</span>
    </button>

    <!-- Paste -->
    <button
      class="flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-foreground transition-colors hover:bg-muted"
      onclick={pasteFromClipboard}
    >
      <span class="flex items-center gap-2">
        <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M6 3h4M5 3a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1" stroke-linecap="round"/>
          <path d="M6 3V2h4v1" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        Paste
      </span>
      <span class="text-[10px] text-muted-foreground/60">Ctrl+V</span>
    </button>

    <div class="my-1 border-t border-border"></div>

    <!-- Clear -->
    <button
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-foreground transition-colors hover:bg-muted"
      onclick={clearAndClose}
    >
      <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M2 4h12M6 4V2h4v2M5 4l1 10h4l1-10" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Clear terminal
    </button>
  </div>
{/if}
