<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import type { ConsoleLog } from "../composables/useConsoleLogs";

const props = defineProps<{
  isOpen: boolean;
  logs: ConsoleLog[];
}>();

const emit = defineEmits<{
  (e: "update:isOpen", val: boolean): void;
}>();

const logsContainer = ref<HTMLDivElement | null>(null);

const toggle = () => {
  emit("update:isOpen", !props.isOpen);
};

// 监听日志的变化以处理容器滚动
watch(
  () => props.logs.length,
  () => {
    if (props.isOpen) {
      nextTick(() => {
        if (logsContainer.value) {
          logsContainer.value.scrollTop = logsContainer.value.scrollHeight;
        }
      });
    }
  }
);
</script>

<template>
  <!-- Bottom Collapsible Global Console Terminal -->
  <div 
    class="absolute bottom-0 left-0 right-0 border-t border-app-border bg-app-surface/90 backdrop-blur-md flex flex-col transition-all duration-500 z-20"
    :style="{ height: isOpen ? '60%' : '44px' }"
  >
    <!-- Console Header Toggle -->
    <div 
      @click="toggle" 
      class="h-11 px-8 border-b border-app-border/40 flex justify-between items-center cursor-pointer hover:bg-app-bg/50 transition-colors"
    >
      <div class="flex items-center gap-2 text-xs font-black text-app-text-dim">
        <!-- Terminal icon -->
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
        <span>全局控制台日志 (实时监控)</span>
      </div>
      
      <div class="flex items-center gap-4">
        <span class="text-[10px] text-app-text-mute font-medium">点击{{ isOpen ? '折叠' : '展开' }}</span>
        <div class="transition-transform duration-300" :class="{ 'rotate-180': isOpen }">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7" />
          </svg>
        </div>
      </div>
    </div>

    <!-- Console Body Logs -->
    <div 
      ref="logsContainer"
      id="console-logs-container"
      class="flex-1 p-6 overflow-y-auto space-y-1.5 custom-scrollbar bg-slate-950 font-mono text-[10px] text-slate-300 leading-relaxed select-text"
      v-show="isOpen"
    >
      <div v-if="logs.length === 0" class="text-slate-500 italic">
        系统控制台就绪，当前暂无操作日志。
      </div>
      <div 
        v-for="(log, i) in logs" 
        :key="i"
        class="flex gap-3 hover:bg-slate-900/50 py-0.5 px-1 rounded transition-colors"
      >
        <span class="text-slate-500 shrink-0">{{ log.time }}</span>
        <span class="text-app-primary shrink-0 font-bold">[{{ log.taskName }}]</span>
        <span 
          :class="{
            'text-slate-200': log.type === 'info',
            'text-app-emerald font-semibold': log.type === 'success',
            'text-app-rose font-semibold': log.type === 'error'
          }"
          class="break-all"
        >
          {{ log.message }}
        </span>
      </div>
    </div>
  </div>
</template>
