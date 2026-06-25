<script lang="ts">
  import { themeStore, setTheme, type ThemePreference } from '$lib/stores/theme.svelte'

  const options: { value: ThemePreference; label: string; title: string }[] = [
    { value: 'light',  label: '☀',  title: 'Light theme' },
    { value: 'system', label: '⬤', title: 'System theme' },
    { value: 'dark',   label: '☾',  title: 'Dark theme'  },
  ]
</script>

<div class="flex rounded border border-border text-[10px]" title="Theme">
  {#each options as opt}
    <button
      class="px-2 py-1 transition-colors {themeStore.preference === opt.value
        ? 'bg-muted text-foreground'
        : 'text-muted-foreground hover:text-foreground'}"
      title={opt.title}
      onclick={() => setTheme(opt.value)}
    >{opt.label}</button>
  {/each}
</div>
