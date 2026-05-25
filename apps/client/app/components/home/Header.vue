<script setup lang="ts">
  import { Globe, Moon, Settings, Sun } from '@lucide/vue';
  import { useHomePage } from '~/composables/use-home-page';

  const { isDark, toggleColorMode, pingQuery } = useHomePage();
</script>

<template>
  <header
    class="sticky top-0 z-30 border-b border-border bg-background/85 backdrop-blur-md"
  >
    <div
      class="mx-auto flex min-h-14 max-w-[1600px] items-center justify-between gap-3 px-4 sm:px-6 lg:px-6"
    >
      <NuxtLink
        to="/"
        class="flex items-center gap-3"
        aria-label="Tileserver RS home"
      >
        <div
          class="flex size-9 items-center justify-center bg-primary"
          aria-hidden="true"
        >
          <Globe class="size-5 text-primary-foreground" />
        </div>
        <div>
          <div class="text-lg font-semibold leading-none tracking-tight">
            Tileserver RS
          </div>
          <div
            v-if="pingQuery.data.value"
            class="text-xs text-muted-foreground"
          >
            v{{ pingQuery.data.value.version }} · MCP
          </div>
        </div>
      </NuxtLink>

      <div class="flex items-center gap-1">
        <NuxtLink to="/admin" aria-label="Open admin">
          <Button variant="ghost" size="icon" as="span">
            <Settings class="size-5" />
          </Button>
        </NuxtLink>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Toggle color mode"
          @click="toggleColorMode"
        >
          <Sun v-if="isDark" class="size-5" />
          <Moon v-else class="size-5" />
        </Button>
      </div>
    </div>
  </header>
</template>
