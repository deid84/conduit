<script lang="ts">
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import type { IDisposable, ITheme } from '@xterm/xterm'
  import type { Connection, LogEntry } from '$lib/stores/connections.svelte'
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

  let container = $state<HTMLDivElement | null>(null)
  let term      = $state<Terminal | null>(null)
  let written   = 0  // local counter — not reactive, just a cursor into connection.log

  function writeEntry(t: Terminal, entry: LogEntry) {
    if (entry.direction === 'tx') t.write('\x1b[36m')
    t.write(new Uint8Array(entry.raw))
    if (entry.direction === 'tx') t.write('\x1b[0m')
  }

  // Effect 1: create the terminal once the container div is in the DOM.
  // disableStdin starts true; Effect 3 enables it in raw mode.
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

    const fitAddon  = new FitAddon()
    const linksAddon = new WebLinksAddon()
    t.loadAddon(fitAddon)
    t.loadAddon(linksAddon)
    t.open(container)
    fitAddon.fit()

    const ro = new ResizeObserver(() => fitAddon.fit())
    ro.observe(container)

    term = t

    return () => {
      ro.disconnect()
      t.dispose()
      term    = null
      written = 0
    }
  })

  // Effect 2: write new log entries as they arrive.
  $effect(() => {
    if (!term) return
    const len = connection.log.length  // tracked — re-runs when log grows
    const t   = term
    for (; written < len; written++) {
      writeEntry(t, connection.log[written])
    }
  })

  // Effect 3: raw mode — forward keystrokes directly to the device.
  $effect(() => {
    const mode = connection.terminalMode  // tracked
    const t    = term                     // tracked

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

  // Effect 4: sync xterm theme when user changes light/dark/system preference.
  const xtermTheme = $derived(effectiveTheme() === 'dark' ? THEME_MOCHA : THEME_LATTE)

  $effect(() => {
    const theme = xtermTheme  // tracked via $derived
    const t     = term
    if (!t) return
    t.options.theme = theme
  })
</script>

<div
  bind:this={container}
  class="flex-1 overflow-hidden"
  style="background-color: {xtermTheme.background}"
></div>
