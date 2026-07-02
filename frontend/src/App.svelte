<script lang="ts">
  import TabBar        from '$lib/components/TabBar.svelte'
  import NewConnection from '$lib/components/NewConnection.svelte'
  import TerminalView  from '$lib/components/TerminalView.svelte'
  import ModbusPanel   from '$lib/components/ModbusPanel.svelte'
  import SendBar       from '$lib/components/SendBar.svelte'
  import MacroBar      from '$lib/components/MacroBar.svelte'
  import StatusBar     from '$lib/components/StatusBar.svelte'
  import { store }     from '$lib/stores/connections.svelte'
  import { cn }        from '$lib/utils'

  const showNew = $derived(store.newConnOpen || store.connections.length === 0)

  // Per-connection view: 'terminal' | 'modbus'. Resets to terminal when activeId changes.
  let view = $state<'terminal' | 'modbus'>('terminal')
  $effect(() => { store.activeId; view = 'terminal' })
</script>

<div class="flex h-full flex-col overflow-hidden">
  <TabBar />

  {#if showNew || !store.active}
    <NewConnection />
  {:else}
    <!-- View switcher (only when modbus mode is active) -->
    {#if store.active.modbusMode}
      <div class="flex shrink-0 gap-px border-b border-border bg-muted/20 px-2 pt-1">
        {#each [['terminal', 'Terminal'], ['modbus', 'Modbus']] as [v, label]}
          <button
            onclick={() => view = v as 'terminal' | 'modbus'}
            class={cn(
              'rounded-t px-3 py-1 text-xs font-medium transition-colors',
              view === v
                ? 'bg-background text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >{label}</button>
        {/each}
      </div>
    {/if}

    {#key store.activeId}
      {#if view === 'terminal' || !store.active.modbusMode}
        <TerminalView connection={store.active} />
        <SendBar />
        <MacroBar />
      {:else}
        <ModbusPanel connection={store.active} />
      {/if}
    {/key}

    <StatusBar connection={store.active} />
  {/if}
</div>
