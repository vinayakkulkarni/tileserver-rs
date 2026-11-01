<template>
  <section class="border-b border-border px-page-x py-6 sm:py-8">
    <div class="max-w-2xl border border-border px-5 py-5 sm:px-6 sm:py-6">
      <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
        Collection is off
      </p>
      <p class="mt-3 text-sm text-foreground">
        Request statistics are not being recorded. Turn them on in the config
        and reload:
      </p>
      <pre
        class="mt-3 border border-border bg-card px-4 py-3 font-mono text-sm text-foreground"
      >
[telemetry]
dashboard = true</pre>
      <p class="mt-3 text-sm text-muted-foreground">
        The data stays in memory and is bounded; it is lost on restart.
      </p>
    </div>
  </section>
</template>
