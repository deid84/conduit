<script lang="ts">
  import { connect } from '$lib/stores/connections.svelte'
  import type { ConnConfig, SerialConfig, TcpConfig, UdpConfig } from '$lib/stores/connections.svelte'
  import { listPorts } from '$lib/api'
  import { cn } from '$lib/utils'

  type Protocol = 'serial' | 'tcp' | 'udp'
  let proto = $state<Protocol>('serial')

  let serial = $state<SerialConfig>({
    port:         '',
    baud_rate:    115200,
    data_bits:    8,
    stop_bits:    1,
    parity:       'none',
    flow_control: 'none',
  })

  let tcp = $state<TcpConfig>({ host: '127.0.0.1', port: 3000 })
  let udp = $state<UdpConfig>({ bind: '0.0.0.0:0', remote: '127.0.0.1:5005' })

  let ports   = $state<string[]>([])
  let loading = $state(false)
  let error   = $state<string | null>(null)

  const BAUD_RATES = [300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1000000, 2000000]

  async function fetchPorts() {
    try {
      const list = await listPorts()
      ports = list.map(p => p.name)
      if (ports.length > 0 && !serial.port) serial.port = ports[0]
    } catch { /* backend not running yet */ }
  }

  $effect(() => { fetchPorts() })

  async function doConnect() {
    error = null
    loading = true
    try {
      let config: ConnConfig
      switch (proto) {
        case 'serial': config = { type: 'serial', config: { ...serial } }; break
        case 'tcp':    config = { type: 'tcp',    config: { ...tcp    } }; break
        case 'udp':    config = { type: 'udp',    config: { ...udp    } }; break
      }
      await connect(config)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  const PROTO_TABS: { id: Protocol; label: string }[] = [
    { id: 'serial', label: 'Serial' },
    { id: 'tcp',    label: 'TCP'    },
    { id: 'udp',    label: 'UDP'    },
  ]

  const INPUT_CLS  = 'w-full rounded border border-border bg-muted/20 px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground/50 focus:border-primary/60 focus:outline-none'
  const SELECT_CLS = 'w-full rounded border border-border bg-muted/20 px-2 py-1.5 text-xs text-foreground focus:border-primary/60 focus:outline-none'
  const LABEL_CLS  = 'mb-1 block text-[11px] text-muted-foreground'
</script>

<div class="flex flex-1 items-center justify-center overflow-hidden bg-background p-8">
  <div class="w-full max-w-md rounded-lg border border-border bg-muted/10 p-6">
    <h2 class="mb-5 text-sm font-semibold uppercase tracking-widest text-foreground">
      New Connection
    </h2>

    <!-- Protocol selector -->
    <div class="mb-6 flex gap-1 rounded-md border border-border bg-muted/30 p-1">
      {#each PROTO_TABS as tab}
        <button
          class={cn(
            'flex-1 rounded py-1 text-xs font-medium transition-colors',
            proto === tab.id
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground',
          )}
          onclick={() => proto = tab.id}
        >{tab.label}</button>
      {/each}
    </div>

    {#if proto === 'serial'}
      <div class="space-y-3">
        <div class="flex gap-2">
          <div class="flex-1">
            <label for="s-port" class={LABEL_CLS}>Port</label>
            <div class="flex gap-1">
              {#if ports.length > 0}
                <select id="s-port" class={cn(SELECT_CLS, 'flex-1')} bind:value={serial.port}>
                  {#each ports as p}
                    <option value={p}>{p}</option>
                  {/each}
                </select>
              {:else}
                <input
                  id="s-port"
                  class={cn(INPUT_CLS, 'flex-1')}
                  bind:value={serial.port}
                  placeholder="COM3 or /dev/ttyUSB0"
                />
              {/if}
              <button
                class="rounded border border-border px-2 text-muted-foreground transition-colors hover:text-foreground"
                title="Refresh ports"
                onclick={fetchPorts}
              >↺</button>
            </div>
          </div>
          <div>
            <label for="s-baud" class={LABEL_CLS}>Baud Rate</label>
            <select id="s-baud" class={SELECT_CLS.replace('w-full', '')} bind:value={serial.baud_rate}>
              {#each BAUD_RATES as rate}
                <option value={rate}>{rate}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="flex gap-2">
          <div class="flex-1">
            <label for="s-data" class={LABEL_CLS}>Data Bits</label>
            <select id="s-data" class={SELECT_CLS} bind:value={serial.data_bits}>
              {#each [5, 6, 7, 8] as n}<option value={n}>{n}</option>{/each}
            </select>
          </div>
          <div class="flex-1">
            <label for="s-stop" class={LABEL_CLS}>Stop Bits</label>
            <select id="s-stop" class={SELECT_CLS} bind:value={serial.stop_bits}>
              {#each [1, 2] as n}<option value={n}>{n}</option>{/each}
            </select>
          </div>
          <div class="flex-1">
            <label for="s-par" class={LABEL_CLS}>Parity</label>
            <select id="s-par" class={SELECT_CLS} bind:value={serial.parity}>
              <option value="none">None</option>
              <option value="odd">Odd</option>
              <option value="even">Even</option>
            </select>
          </div>
        </div>

        <div>
          <label for="s-flow" class={LABEL_CLS}>Flow Control</label>
          <select id="s-flow" class={SELECT_CLS} bind:value={serial.flow_control}>
            <option value="none">None</option>
            <option value="software">Software (XON/XOFF)</option>
            <option value="hardware">Hardware (RTS/CTS)</option>
          </select>
        </div>
      </div>

    {:else if proto === 'tcp'}
      <div class="space-y-3">
        <div>
          <label for="t-host" class={LABEL_CLS}>Host</label>
          <input id="t-host" class={INPUT_CLS} bind:value={tcp.host} placeholder="127.0.0.1" />
        </div>
        <div>
          <label for="t-port" class={LABEL_CLS}>Port</label>
          <input id="t-port" type="number" class={INPUT_CLS} bind:value={tcp.port} min="1" max="65535" />
        </div>
      </div>

    {:else}
      <div class="space-y-3">
        <div>
          <label for="u-bind" class={LABEL_CLS}>Bind (local)</label>
          <input id="u-bind" class={INPUT_CLS} bind:value={udp.bind} placeholder="0.0.0.0:0" />
        </div>
        <div>
          <label for="u-remote" class={LABEL_CLS}>Remote</label>
          <input id="u-remote" class={INPUT_CLS} bind:value={udp.remote} placeholder="192.168.1.10:5005" />
        </div>
      </div>
    {/if}

    {#if error}
      <p class="mt-3 rounded border border-red-800/50 bg-red-950/30 px-3 py-2 text-[11px] text-red-400">{error}</p>
    {/if}

    <button
      class="mt-6 w-full rounded bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 active:opacity-75 disabled:cursor-not-allowed disabled:opacity-40"
      disabled={loading || (proto === 'serial' && !serial.port.trim())}
      onclick={doConnect}
    >{loading ? 'Connecting…' : 'Connect'}</button>
  </div>
</div>
