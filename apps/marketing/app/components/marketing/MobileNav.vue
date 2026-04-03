<script setup lang="ts">
  import { Github, Globe, Menu, Moon, Sun, X } from 'lucide-vue-next';

  const { isDark, toggle } = useThemeToggle();

  const isOpen = ref(false);

  function handleClose() {
    isOpen.value = false;
  }

  if (import.meta.client) {
    const isLocked = useScrollLock(document.body);
    watch(isOpen, (val) => {
      isLocked.value = val;
    });
  }

  const links = [
    { label: 'Docs', href: 'https://docs.tileserver.app' },
    { label: 'Demo', href: 'https://demo.tileserver.app', external: true },
    {
      label: 'GitHub',
      href: 'https://github.com/vinayakkulkarni/tileserver-rs',
      icon: Github,
      external: true,
    },
  ];
</script>

<template>
  <div class="md:hidden">
    <button
      class="
        flex items-center justify-center p-2
        hover:bg-accent
      "
      aria-label="Open menu"
      @click="isOpen = true"
    >
      <Menu class="size-5" />
    </button>

    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="isOpen"
          class="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm"
          @click="handleClose"
        ></div>
      </Transition>

      <Transition name="slide-left">
        <div
          v-if="isOpen"
          class="
            fixed inset-y-0 left-0 z-100 flex w-72 flex-col border-r
            border-border bg-background shadow-2xl
          "
        >
          <div
            class="
              flex items-center justify-between border-b border-border px-4 py-3
            "
          >
            <div class="flex items-center gap-2.5">
              <Globe class="size-5 text-primary" />
              <span
                class="
                  font-display text-sm font-semibold tracking-[0.15em] uppercase
                "
              >
                <span class="text-foreground">Tileserver</span>
                <span class="text-primary"> RS</span>
              </span>
            </div>
            <button
              class="
                flex items-center justify-center p-2
                hover:bg-accent
              "
              aria-label="Close menu"
              @click="handleClose"
            >
              <X class="size-5" />
            </button>
          </div>

          <nav class="flex-1 overflow-y-auto px-4 py-6">
            <ul class="space-y-1">
              <li v-for="link in links" :key="link.label">
                <a
                  :href="link.href"
                  :target="link.external ? '_blank' : undefined"
                  :rel="link.external ? 'noopener noreferrer' : undefined"
                  class="
                    flex items-center gap-2 px-3 py-2.5 font-mono text-sm
                    tracking-wider text-muted-foreground uppercase
                    hover:bg-accent hover:text-foreground
                  "
                  @click="handleClose"
                >
                  <component :is="link.icon" v-if="link.icon" class="size-4" />
                  {{ link.label }}
                </a>
              </li>
            </ul>
          </nav>

          <div class="border-t border-border p-4">
            <button
              class="
                flex w-full items-center gap-2 px-3 py-2.5 font-mono text-sm
                tracking-wider text-muted-foreground uppercase
                hover:bg-accent hover:text-foreground
              "
              @click="toggle"
            >
              <Sun v-if="isDark" class="size-4" />
              <Moon v-else class="size-4" />
              {{ isDark ? 'Light mode' : 'Dark mode' }}
            </button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
  .fade-enter-active,
  .fade-leave-active {
    transition: opacity 0.15s ease;
  }
  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }

  .slide-left-enter-active,
  .slide-left-leave-active {
    transition: transform 0.2s ease-out;
  }
  .slide-left-enter-from,
  .slide-left-leave-to {
    transform: translateX(-100%);
  }
</style>
