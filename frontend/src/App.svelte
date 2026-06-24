<script>
  let status = $state('ready')
</script>

<main>
  <header>
    <h1>Conduit</h1>
    <span class="status">{status}</span>
  </header>
  <p class="subtitle">Serial / TCP / UDP monitor</p>
</main>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: system-ui, -apple-system, sans-serif;
    background: #0f1117;
    color: #e2e8f0;
    height: 100vh;
    overflow: hidden;
  }

  main {
    padding: 1.5rem;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    font-size: 1.1rem;
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .status {
    font-size: 0.7rem;
    color: #4ade80;
    border: 1px solid #4ade80;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
  }

  .subtitle {
    font-size: 0.85rem;
    color: #64748b;
  }
</style>
