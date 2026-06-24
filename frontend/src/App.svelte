<script lang="ts">
  let status = $state<'ready' | 'connected' | 'error'>('ready')
</script>

<div class="flex h-full flex-col">
  <header class="flex items-center gap-3 border-b border-border px-4 py-2">
    <h1 class="text-sm font-semibold tracking-widest text-foreground uppercase">
      Conduit
    </h1>
    <span class="rounded-full border border-primary/40 px-2 py-0.5 text-[10px] text-primary">
      {status}
    </span>
  </header>

  <main class="flex-1 overflow-hidden p-4">
    <p class="text-sm text-muted-foreground">Serial / TCP / UDP monitor</p>
  </main>
</div>
