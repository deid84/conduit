<script lang="ts">
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import type { Connection, LogEntry } from '$lib/stores/connections.svelte'
  import '@xterm/xterm/css/xterm.css'

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
  $effect(() => {
    if (!container) return

    const t = new Terminal({
      cursorBlink:  true,
      cursorStyle:  'bar',
      fontFamily:   '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
      fontSize:     12,
      lineHeight:   1.4,
      convertEol:   true,
      scrollback:   10_000,
      theme: {
        background:          '#09090b',
        foreground:          '#e4e4e7',
        cursor:              '#a1a1aa',
        cursorAccent:        '#09090b',
        selectionBackground: '#3f3f46',
        black:               '#18181b',
        red:                 '#f87171',
        green:               '#4ade80',
        yellow:              '#fbbf24',
        blue:                '#60a5fa',
        magenta:             '#c084fc',
        cyan:                '#22d3ee',
        white:               '#e4e4e7',
        brightBlack:         '#52525b',
        brightRed:           '#fca5a5',
        brightGreen:         '#86efac',
        brightYellow:        '#fde68a',
        brightBlue:          '#93c5fd',
        brightMagenta:       '#d8b4fe',
        brightCyan:          '#67e8f9',
        brightWhite:         '#f4f4f5',
      },
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
</script>

<div bind:this={container} class="flex-1 overflow-hidden bg-[#09090b]"></div>
