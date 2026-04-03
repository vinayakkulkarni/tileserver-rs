<script setup lang="ts">
  import { Globe } from 'lucide-vue-next';
</script>

<template>
  <footer>
    <div
class="
  flex items-center justify-between px-5 py-6
  lg:px-8
">
      <div class="flex items-center gap-3">
        <Globe class="size-4 text-muted-foreground" />
        <span
          class="
            font-mono text-[10px] tracking-wider text-muted-foreground uppercase
            lg:text-xs
          "
        >
          &copy; {{ new Date().getFullYear() }}
          <NuxtLink
            to="https://vinayakkulkarni.dev"
            external
            class="
              transition-colors
              hover:text-foreground
            "
            >Vinayak Kulkarni</NuxtLink
          >
        </span>
      </div>
      <div class="flex items-center gap-4">
        <NuxtLink
          to="https://docs.tileserver.app"
          external
          class="
            font-mono text-[10px] tracking-wider text-muted-foreground uppercase
            transition-colors
            hover:text-foreground
            lg:text-xs
          "
        >
          Docs
        </NuxtLink>
        <NuxtLink
          to="https://github.com/vinayakkulkarni/tileserver-rs"
          external
          class="
            font-mono text-[10px] tracking-wider text-muted-foreground uppercase
            transition-colors
            hover:text-foreground
            lg:text-xs
          "
        >
          GitHub
        </NuxtLink>
        <NuxtLink
          to="https://github.com/vinayakkulkarni/tileserver-rs/blob/main/LICENSE"
          external
          class="
            font-mono text-[10px] tracking-wider text-muted-foreground uppercase
            transition-colors
            hover:text-foreground
            lg:text-xs
          "
        >
          MIT License
        </NuxtLink>
      </div>
    </div>
  </footer>
</template>
