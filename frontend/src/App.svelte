<script lang="ts">
  import TabBar        from '$lib/components/TabBar.svelte'
  import NewConnection from '$lib/components/NewConnection.svelte'
  import TerminalView  from '$lib/components/TerminalView.svelte'
  import SendBar       from '$lib/components/SendBar.svelte'
  import MacroBar      from '$lib/components/MacroBar.svelte'
  import StatusBar     from '$lib/components/StatusBar.svelte'
  import { store }     from '$lib/stores/connections.svelte'

  const showNew = $derived(store.newConnOpen || store.connections.length === 0)
</script>

<div class="flex h-full flex-col overflow-hidden">
  <TabBar />

  {#if showNew || !store.active}
    <NewConnection />
  {:else}
    {#key store.activeId}
      <TerminalView connection={store.active} />
    {/key}
    <SendBar />
    <MacroBar />
    <StatusBar connection={store.active} />
  {/if}
</div>
