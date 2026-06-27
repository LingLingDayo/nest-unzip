<script setup lang="ts">
import { ref, onMounted } from "vue";

import SettingsModal from "./components/common/SettingsModal/SettingsModal.vue";
import PasswordModal from "./components/common/PasswordModal.vue";
import TaskCard from "./components/TaskCard.vue";
import ConsolePanel from "./components/ConsolePanel.vue";

import { useSettings, settingGroups } from "./composables/useSettings";
import { useUnzip } from "./composables/useUnzip";
import { useConsoleLogs } from "./composables/useConsoleLogs";
import pkg from "../package.json";

const passwordModalRef = ref<InstanceType<typeof PasswordModal> | null>(null);
const showSettings = ref(false);

// 1. 实例化全局日志状态
const { consoleLogs, isConsoleOpen, addLog, clearLogs } = useConsoleLogs();

// 2. 实例化设置模块
const {
  appSettings,
  detectedToolsState,
  loadSettings,
  saveSettings,
} = useSettings(addLog);

// 3. 实例化解压缩引擎与任务模块
const {
  tasks,
  isProcessing,
  handleSelectFiles,
  handleSelectTargetDir,
  removeTask,
  clearTasks,
  startBulkExtraction,
} = useUnzip(appSettings, detectedToolsState, passwordModalRef, addLog);

onMounted(async () => {
  await loadSettings();
  addLog("系统", "NestUnzip 深度解压已启动。请拖入或点击添加压缩包。");
});
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-app-bg text-app-text font-sans overflow-hidden select-none relative">
    
    <!-- Title Header -->
    <header class="h-16 border-b border-app-border bg-app-surface/60 backdrop-blur-md px-8 flex items-center justify-between z-10 shrink-0">
      <div class="flex items-center gap-3">
        <!-- Main Logo / Icon -->
        <div class="w-9 h-9 rounded-2xl bg-gradient-to-tr from-app-primary to-app-purple flex items-center justify-center shadow-lg shadow-app-primary/20">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
          </svg>
        </div>
        <div>
          <h1 class="text-lg font-black tracking-tight text-app-text flex items-center gap-2">
            NestUnzip <span class="text-[10px] font-black uppercase text-app-primary bg-app-primary-light px-2 py-0.5 rounded-full tracking-wider border border-app-primary/10">v{{ pkg.version }}</span>
          </h1>
          <p class="text-[10px] text-app-text-mute font-medium">深度嵌套批量解压专家</p>
        </div>
      </div>

      <!-- Engine Status & Setting Button -->
      <div class="flex items-center gap-6">
        <div class="flex items-center gap-4 text-xs font-semibold">
          <!-- 7z status -->
          <div class="flex items-center gap-1.5" :class="detectedToolsState.sevenZip ? 'text-app-emerald' : 'text-app-text-mute opacity-60'">
            <span class="w-1.5 h-1.5 rounded-full" :class="detectedToolsState.sevenZip ? 'bg-app-emerald' : 'bg-app-text-mute'"></span>
            <span>7-Zip</span>
          </div>
          <!-- bandizip status -->
          <div class="flex items-center gap-1.5" :class="detectedToolsState.bandizip ? 'text-app-emerald' : 'text-app-text-mute opacity-60'">
            <span class="w-1.5 h-1.5 rounded-full" :class="detectedToolsState.bandizip ? 'bg-app-emerald' : 'bg-app-text-mute'"></span>
            <span>Bandizip</span>
          </div>
        </div>

        <button 
          @click="showSettings = true" 
          class="p-2.5 rounded-2xl border border-app-border bg-app-surface/80 text-app-text hover:text-app-primary hover:border-app-primary/30 transition-all hover:bg-app-bg shadow-sm active:scale-95 cursor-pointer flex items-center justify-center group"
          title="系统设置"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 group-hover:rotate-45 transition-transform duration-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>
    </header>

    <!-- Main Workspace -->
    <div class="flex-1 flex overflow-hidden p-6 pb-16 gap-6 bg-app-bg/20">
      
      <!-- Left Panel: Task Manager -->
      <div class="flex-1 flex flex-col bg-app-surface border border-app-border rounded-[28px] shadow-app-md overflow-hidden relative">
        
        <!-- Action Toolbar -->
        <div class="px-6 py-4 border-b border-app-border flex justify-between items-center bg-app-surface shrink-0 z-10">
          <div class="flex items-center gap-2">
            <span class="text-sm font-black text-app-text">任务列表</span>
            <span class="text-[11px] font-bold text-app-primary bg-app-primary-light px-2.5 py-0.5 rounded-full">{{ tasks.length }} 个任务</span>
          </div>
          
          <div class="flex items-center gap-3">
            <button 
              @click="handleSelectFiles" 
              class="px-4 py-2 border border-app-border rounded-xl text-xs font-black text-app-text-dim hover:text-app-text hover:bg-app-bg cursor-pointer transition-all active:scale-95 shadow-sm"
              :disabled="isProcessing"
            >
              添加压缩包
            </button>
            <button 
              @click="clearTasks" 
              class="px-4 py-2 border border-app-border rounded-xl text-xs font-black text-app-rose/80 hover:text-app-rose hover:bg-app-rose/5 cursor-pointer transition-all active:scale-95 shadow-sm"
              :disabled="isProcessing"
            >
              清空
            </button>
            <button 
              @click="startBulkExtraction" 
              class="px-5 py-2 bg-gradient-to-r from-app-primary to-app-primary-hover text-white rounded-xl text-xs font-black hover:shadow-lg hover:shadow-app-primary/10 transition-all active:scale-95 flex items-center gap-2 cursor-pointer"
              :class="isProcessing || tasks.length === 0 ? 'opacity-50 pointer-events-none' : ''"
            >
              <svg v-if="isProcessing" class="animate-spin h-3.5 w-3.5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>{{ isProcessing ? '深度解压中...' : '开始批量解压' }}</span>
            </button>
          </div>
        </div>

        <!-- Empty Dropzone -->
        <div 
          v-if="tasks.length === 0" 
          @click="handleSelectFiles"
          class="flex-1 m-8 border-2 border-dashed border-app-border hover:border-app-primary/40 rounded-2xl flex flex-col items-center justify-center cursor-pointer hover:bg-app-primary-light/10 transition-all group duration-300"
        >
          <div class="w-16 h-16 rounded-3xl bg-app-bg group-hover:bg-app-primary-light/50 flex items-center justify-center text-app-text-mute group-hover:text-app-primary transition-all duration-300 shadow-inner">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
            </svg>
          </div>
          <h3 class="mt-4 text-sm font-black text-app-text">拖拽压缩包到此处</h3>
          <p class="mt-2 text-xs text-app-text-mute">支持拖入多个文件或点击选择。支持 zip, 7z, rar, tar 等格式</p>
        </div>

        <!-- Scrollable Task List -->
        <div v-else class="flex-1 overflow-y-auto p-6 space-y-4 custom-scrollbar bg-app-bg/5">
          <TaskCard 
            v-for="(task, index) in tasks" 
            :key="task.id"
            :task="task"
            :is-processing="isProcessing"
            @remove="removeTask(index)"
            @change-dir="handleSelectTargetDir(task)"
          />
        </div>

      </div>

    </div>

    <!-- Mask overlay for bottom drawer -->
    <Transition name="fade">
      <div 
        v-if="isConsoleOpen"
        @click="isConsoleOpen = false"
        class="absolute inset-0 bg-black/20 backdrop-blur-[0.5px] z-[15] cursor-pointer"
      ></div>
    </Transition>

    <!-- Bottom Collapsible Global Console Terminal -->
    <ConsolePanel 
      v-model:is-open="isConsoleOpen"
      :logs="consoleLogs"
      @clear="clearLogs"
    />

    <!-- Settings Modal Dynamic Control -->
    <SettingsModal 
      v-model:show="showSettings" 
      v-model:settings="appSettings"
      :groups="settingGroups"
      @save="saveSettings"
    />

    <!-- Password Modal -->
    <PasswordModal ref="passwordModalRef" />

  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 5px; height: 5px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { 
    background: var(--color-app-border); 
    border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: var(--color-app-text-mute); }

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>