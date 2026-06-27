<script setup lang="ts">
import type { ExtractTask } from "../composables/useUnzip";

defineProps<{
  task: ExtractTask;
  isProcessing: boolean;
}>();

const emit = defineEmits<{
  (e: "remove"): void;
  (e: "change-dir"): void;
  (e: "toggle-select"): void;
}>();
</script>

<template>
  <div 
    @click="isProcessing ? null : emit('toggle-select')"
    class="p-5 rounded-2xl border bg-app-surface shadow-app-sm flex flex-col gap-4 transition-all duration-300 hover:shadow-app-md relative cursor-pointer group"
    :class="task.selected 
      ? 'border-app-primary bg-app-primary-light/5 shadow-md shadow-app-primary/5' 
      : 'border-app-border hover:border-app-border-focus'"
  >
    <!-- Task Info Header -->
    <div class="flex items-start justify-between">
      <div class="flex items-center gap-3 min-w-0">
        <div class="w-10 h-10 rounded-xl bg-app-bg flex items-center justify-center shrink-0 border border-app-border/40">
          <!-- File type specific icon style -->
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-app-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <div class="min-w-0">
          <h4 class="text-xs font-black text-app-text truncate" :title="task.name">{{ task.name }}</h4>
          <p class="text-[10px] text-app-text-mute font-medium truncate mt-0.5" :title="task.path">{{ task.path }}</p>
        </div>
      </div>

      <!-- Task Status Pill -->
      <div class="flex items-center gap-2 shrink-0">
        <span 
          class="text-[9px] font-black uppercase tracking-wider px-2 py-0.5 rounded-full whitespace-nowrap"
          :class="{
            'bg-app-bg text-app-text-mute': task.status === 'pending',
            'bg-app-primary-light text-app-primary animate-pulse': task.status === 'running',
            'bg-app-emerald/10 text-app-emerald': task.status === 'success',
            'bg-app-rose/10 text-app-rose': task.status === 'error'
          }"
        >
          {{ task.status === 'pending' ? '等待中' : task.status === 'running' ? '正在解压' : task.status === 'success' ? '解压完成' : '失败' }}
        </span>
        
        <!-- Action Delete button -->
        <button 
          v-if="!isProcessing"
          @click.stop="emit('remove')" 
          class="text-app-text-mute hover:text-app-rose p-1 rounded-lg hover:bg-app-bg transition-colors cursor-pointer"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Task Parameters Form -->
    <div class="grid grid-cols-2 gap-4 border-t border-app-border/40 pt-4 text-[11px]">
      
      <!-- Target Dir Settings -->
      <div class="flex flex-col gap-1 min-w-0">
        <span class="font-black text-app-text-dim">解压到目录</span>
        <div class="flex gap-2 items-center">
          <span class="text-app-text-mute truncate flex-1 font-medium bg-app-bg/50 px-2 py-1.5 rounded-lg border border-app-border/20" :title="task.targetDir">
            {{ task.targetDir }}
          </span>
          <button 
            @click.stop="emit('change-dir')"
            class="px-2 py-1.5 border border-app-border hover:bg-app-bg text-[10px] font-black rounded-lg text-app-text-dim transition-colors shrink-0 cursor-pointer"
            :disabled="isProcessing"
          >
            更改
          </button>
        </div>
      </div>

      <!-- Password settings -->
      <div class="flex flex-col gap-1">
        <span class="font-black text-app-text-dim flex items-center gap-1">
          <span>解压密码</span>
          <span class="text-[9px] font-medium opacity-50">(可不填，多个用空格隔开)</span>
        </span>
        <input 
          type="password"
          v-model="task.passwords"
          placeholder="优先尝试此处的独立密码"
          class="w-full bg-app-surface border border-app-border rounded-lg px-2.5 py-1.5 text-[11px] font-medium placeholder:text-app-text-mute focus:outline-none focus:border-app-primary/50 transition-colors"
          :disabled="isProcessing"
          @click.stop
        />
      </div>

    </div>

    <!-- Progress Bar -->
    <div v-if="task.status === 'running' || task.status === 'success' || task.status === 'error'" class="w-full space-y-1.5">
      <div class="flex justify-between items-center text-[10px] font-black">
        <span class="text-app-text-dim">解压进度</span>
        <span :class="task.status === 'error' ? 'text-app-rose' : 'text-app-primary'">{{ Math.round(task.progress) }}%</span>
      </div>
      <div class="w-full h-1.5 bg-app-bg rounded-full overflow-hidden">
        <div 
          class="h-full rounded-full transition-all duration-300"
          :class="task.status === 'error' ? 'bg-app-rose' : 'bg-gradient-to-r from-app-primary to-app-purple'"
          :style="{ width: `${task.progress}%` }"
        ></div>
      </div>
    </div>

  </div>
</template>
