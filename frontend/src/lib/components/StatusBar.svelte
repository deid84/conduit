<script lang="ts">
  import type { Connection } from '$lib/stores/connections.svelte'
  import { cn } from '$lib/utils'

  let { connection }: { connection: Connection } = $props()

  function fmtBytes(n: number): string {
    if (n < 1024)       return `${n} B`
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
    return `${(n / 1024 / 1024).toFixed(1)} MB`
  }

  const STATUS_COLOR: Record<string, string> = {
    connected:    'text-green-400',
    connecting:   'text-yellow-400',
    error:        'text-red-400',
    disconnected: 'text-zinc-500',
    idle:         'text-zinc-500',
  }

  const details = $derived((): string => {
    const c = connection.conn
    switch (c.type) {
      case 'serial': {
        const s = c.config
        return `${s.port}  ${s.baud_rate}  ${s.data_bits}${s.parity[0].toUpperCase()}${s.stop_bits}`
      }
      case 'tcp':
        return `${c.config.host}:${c.config.port}`
      case 'udp':
        return `bind ${c.config.bind}  →  ${c.config.remote}`
    }
  })
</script>

<div class="flex shrink-0 items-center gap-4 border-t border-border bg-muted/10 px-3 py-1 text-[11px]">
  <span class={cn('font-medium capitalize', STATUS_COLOR[connection.status])}>
    ● {connection.status}
  </span>

  <span class="text-muted-foreground">{details()}</span>

  <span class="ml-auto text-muted-foreground">RX {fmtBytes(connection.rxBytes)}</span>
  <span class="text-muted-foreground">TX {fmtBytes(connection.txBytes)}</span>
</div>
