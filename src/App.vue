<script setup lang="ts">
import { ref, onMounted, onUnmounted, reactive, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

import SettingsModal from "./components/common/SettingsModal/SettingsModal.vue";
import type { SettingGroup } from "./components/common/SettingsModal/types";
import { extractDefaultSettings } from "./components/common/SettingsModal/utils";

// ==========================================
// 1. 任务定义与状态
// ==========================================
interface ExtractTask {
  id: string;
  name: string;
  path: string;
  status: "pending" | "running" | "success" | "error";
  progress: number;
  passwords: string; // 专属密码，逗号隔开
  targetDir: string; // 自定义解压输出目录
  log: string[];
}

const tasks = ref<ExtractTask[]>([]);
const isProcessing = ref(false);
const showSettings = ref(false);
const activeLogTaskId = ref<string | null>(null);

// 全局日志终端内容
interface ConsoleLog {
  time: string;
  taskName: string;
  message: string;
  type: "info" | "success" | "error";
}
const consoleLogs = ref<ConsoleLog[]>([]);
const isConsoleOpen = ref(true);

const addLog = (taskName: string, message: string, type: "info" | "success" | "error" = "info") => {
  const time = new Date().toLocaleTimeString();
  consoleLogs.value.push({ time, taskName, message, type });
  
  // 保持日志数量不要无限增长
  if (consoleLogs.value.length > 500) {
    consoleLogs.value.shift();
  }

  // 滚动到底部
  setTimeout(() => {
    const el = document.getElementById("console-logs-container");
    if (el) el.scrollTop = el.scrollHeight;
  }, 50);
};

// ==========================================
// 2. 设置弹窗数据配置
// ==========================================
const settingGroups: SettingGroup[] = [
  {
    id: "tools",
    title: "解压引擎配置",
    description: "配置 7-Zip 或 Bandizip 的路径及首选解压工具。",
    color: "#0ea5e9",
    items: [
      {
        id: "sevenZipPath",
        type: "input",
        label: "7-Zip 可执行文件路径 (7z.exe)",
        description: "通常位于 C:\\Program Files\\7-Zip\\7z.exe，为空则默认自动检测",
        defaultValue: "",
      },
      {
        id: "bandizipPath",
        type: "input",
        label: "Bandizip 命令行路径 (bc.exe)",
        description: "通常位于 C:\\Program Files\\Bandizip\\bc.exe，为空则默认自动检测",
        defaultValue: "",
      },
      {
        id: "preferredTool",
        type: "select",
        label: "首选解压引擎",
        description: "系统将优先使用该工具执行解压缩任务。",
        defaultValue: "7z",
        options: [
          { label: "7-Zip (7z.exe)", value: "7z" },
          { label: "Bandizip (bc.exe)", value: "bandizip" },
        ],
      },
    ],
  },
  {
    id: "general",
    title: "通用解包设置",
    description: "配置全局默认尝试密码列表、安全限制等。",
    color: "#8b5cf6",
    items: [
      {
        id: "globalPasswords",
        type: "textarea",
        label: "默认全局解压密码列表",
        description: "每行输入一个密码。解包遇到加密时，程序会依次自动尝试这些密码。",
        defaultValue: "",
        placeholder: "123456\npassword\nadmin",
      },
      {
        id: "autoOpen",
        type: "switch",
        label: "解压完成后自动打开目标文件夹",
        description: "任务成功解压完毕后，在文件资源管理器中打开目标目录。",
        defaultValue: true,
      },
    ],
  },
];

let appSettings = reactive(extractDefaultSettings(settingGroups));

// 工具状态指示
const detectedToolsState = reactive({
  sevenZip: false,
  bandizip: false,
});

// 加载和保存配置
const loadSettings = async () => {
  const saved = localStorage.getItem("unzip_nest_settings");
  if (saved) {
    try {
      const parsed = JSON.parse(saved);
      Object.assign(appSettings, parsed);
    } catch (e) {
      console.error(e);
    }
  }

  // 调用 Rust 后端自动检测工具
  try {
    const detected: any = await invoke("detect_tools");
    detectedToolsState.sevenZip = !!detected.seven_zip;
    detectedToolsState.bandizip = !!detected.bandizip;

    // 如果还没有手动配置过路径，自动填入检测到的路径
    if (!appSettings.sevenZipPath && detected.seven_zip) {
      appSettings.sevenZipPath = detected.seven_zip;
    }
    if (!appSettings.bandizipPath && detected.bandizip) {
      appSettings.bandizipPath = detected.bandizip;
    }
    
    // 如果首选引擎还未设置过，或者首选的工具不可用，自动推荐一个可用的
    if (!saved) {
      if (detected.seven_zip) {
        appSettings.preferredTool = "7z";
      } else if (detected.bandizip) {
        appSettings.preferredTool = "bandizip";
      }
    }
  } catch (e) {
    addLog("系统", `初始化工具检测失败: ${e}`, "error");
  }
};

const saveSettings = (newSettings: any) => {
  Object.assign(appSettings, newSettings);
  localStorage.setItem("unzip_nest_settings", JSON.stringify(appSettings));
  addLog("系统", "设置已保存更改");
};

// ==========================================
// 3. 文件解析与添加
// ==========================================
function parsePath(fullPath: string) {
  const normalized = fullPath.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  const parentDir = lastSlash > -1 ? fullPath.substring(0, lastSlash) : "";
  const filename = lastSlash > -1 ? fullPath.substring(lastSlash + 1) : fullPath;
  
  const lastDot = filename.lastIndexOf(".");
  const baseName = lastDot > -1 ? filename.substring(0, lastDot) : filename;
  
  const separator = fullPath.includes("\\") ? "\\" : "/";
  const targetDir = parentDir ? `${parentDir}${separator}${baseName}` : baseName;
  
  return { filename, targetDir };
}

const addFilesByPaths = (filePaths: string[]) => {
  const extensions = [".zip", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz"];
  let addedCount = 0;

  filePaths.forEach((p) => {
    const { filename, targetDir } = parsePath(p);
    const isArchive = extensions.some((ext) => filename.toLowerCase().endsWith(ext));
    
    // 查重
    const exists = tasks.value.some((t) => t.path === p);

    if (isArchive && !exists) {
      tasks.value.push({
        id: Math.random().toString(36).substring(2, 9),
        name: filename,
        path: p,
        status: "pending",
        progress: 0,
        passwords: "",
        targetDir: targetDir,
        log: [],
      });
      addedCount++;
    }
  });

  if (addedCount > 0) {
    addLog("系统", `成功添加 ${addedCount} 个压缩包文件`);
  }
};

const handleSelectFiles = async () => {
  try {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Compressed Archives",
          extensions: ["zip", "7z", "rar", "tar", "gz", "bz2", "xz"],
        },
      ],
    });
    if (selected && Array.isArray(selected)) {
      addFilesByPaths(selected);
    } else if (selected && typeof selected === "string") {
      addFilesByPaths([selected]);
    }
  } catch (e) {
    addLog("系统", `选择文件失败: ${e}`, "error");
  }
};

const handleSelectTargetDir = async (task: ExtractTask) => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === "string") {
      task.targetDir = selected;
      addLog(task.name, `解压目录已更改为: ${selected}`);
    }
  } catch (e) {
    addLog("系统", `更改目录失败: ${e}`, "error");
  }
};

const removeTask = (index: number) => {
  const t = tasks.value[index];
  tasks.value.splice(index, 1);
  addLog("系统", `移除了任务: ${t.name}`);
};

const clearTasks = () => {
  if (isProcessing.value) return;
  tasks.value = [];
  consoleLogs.value = [];
  addLog("系统", "列表与日志已清空");
};

// ==========================================
// 4. 执行解压缩
// ==========================================
let unlistenLog: UnlistenFn | null = null;
let unlistenDragDrop: UnlistenFn | null = null;

const startBulkExtraction = async () => {
  if (isProcessing.value || tasks.value.length === 0) return;

  // 检查引擎路径配置
  const exeType = appSettings.preferredTool;
  const exePath = exeType === "7z" ? appSettings.sevenZipPath : appSettings.bandizipPath;

  if (!exePath) {
    showSettings.value = true;
    addLog("系统", `错误: 未配置或未检测到 ${exeType === "7z" ? "7-Zip" : "Bandizip"} 的可执行程序路径。请先在设置中配置！`, "error");
    alert(`请先在右上角设置中配置 ${exeType === "7z" ? "7-Zip" : "Bandizip"} 的安装路径！`);
    return;
  }

  isProcessing.value = true;
  addLog("系统", `批量深度解压启动，引擎: ${exeType === "7z" ? "7-Zip" : "Bandizip"}...`);

  // 顺序执行
  for (let i = 0; i < tasks.value.length; i++) {
    const task = tasks.value[i];
    if (task.status === "success") continue; // 跳过已成功的

    task.status = "running";
    task.progress = 5;
    activeLogTaskId.value = task.id;

    // 合并密码列表
    // 全局密码按行拆分
    const globalPwds = (appSettings.globalPasswords as string)
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    
    // 专属密码按逗号/空格拆分
    const taskPwds = task.passwords
      .split(/[,,，\s]/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    const mergedPasswords = Array.from(new Set([...taskPwds, ...globalPwds]));

    addLog(task.name, "解压缩流程初始化...");

    try {
      await invoke("run_depth_extraction", {
        taskId: task.id,
        archivePath: task.path,
        targetDir: task.targetDir,
        passwords: mergedPasswords,
        exePath: exePath,
        exeType: exeType,
      });

      // 异步等待当前任务执行结束
      await new Promise<void>((resolve) => {
        const checkStatus = setInterval(() => {
          const updatedTask = tasks.value.find((t) => t.id === task.id);
          if (updatedTask && (updatedTask.status === "success" || updatedTask.status === "error")) {
            clearInterval(checkStatus);
            resolve();
          }
        }, 500);
      });
    } catch (e) {
      task.status = "error";
      task.progress = 100;
      addLog(task.name, `调用后台任务失败: ${e}`, "error");
    }
  }

  isProcessing.value = false;
  addLog("系统", "批量深度解压缩全部任务已完成！");
};

// ==========================================
// 5. 生命周期与监听
// ==========================================
onMounted(async () => {
  await loadSettings();
  addLog("系统", "NestUnzip 深度解压已启动。请拖入或点击添加压缩包。");

  // 监听后端传递回来的日志事件
  unlistenLog = await listen("extract-log", (event: any) => {
    const { task_id, message, status, progress } = event.payload;
    const task = tasks.value.find((t) => t.id === task_id);

    if (task) {
      task.progress = progress;
      task.status = status;
      task.log.push(message);

      let logType: "info" | "success" | "error" = "info";
      if (status === "success") {
        logType = "success";
        addLog(task.name, `[成功] ${message}`, logType);
        
        // 自动打开文件夹
        if (appSettings.autoOpen) {
          openPath(task.targetDir).catch((err) => {
            addLog(task.name, `自动打开目标文件夹失败: ${err}`, "error");
          });
        }
      } else if (status === "error") {
        logType = "error";
        addLog(task.name, `[失败] ${message}`, logType);
      } else {
        addLog(task.name, message, logType);
      }
    }
  });

  // 监听窗口的拖拽文件事件
  unlistenDragDrop = await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === "drop") {
      addFilesByPaths(event.payload.paths);
    }
  });
});

onUnmounted(() => {
  if (unlistenLog) unlistenLog();
  if (unlistenDragDrop) unlistenDragDrop();
});

const activeLogTask = computed(() => {
  return tasks.value.find((t) => t.id === activeLogTaskId.value);
});
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-app-bg text-app-text font-sans overflow-hidden select-none select-none">
    
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
            NestUnzip <span class="text-[10px] font-black uppercase text-app-primary bg-app-primary-light px-2 py-0.5 rounded-full tracking-wider border border-app-primary/10">v2.1</span>
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
    <div class="flex-1 flex overflow-hidden p-6 gap-6 bg-app-bg/20">
      
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
          <div 
            v-for="(task, index) in tasks" 
            :key="task.id"
            @click="activeLogTaskId = task.id"
            class="p-5 rounded-2xl border bg-app-surface shadow-app-sm flex flex-col gap-4 transition-all duration-300 hover:shadow-app-md cursor-pointer relative"
            :class="[
              activeLogTaskId === task.id ? 'border-app-primary ring-2 ring-app-primary/5 bg-app-primary-light/5' : 'border-app-border hover:border-app-border-focus'
            ]"
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
              <div class="flex items-center gap-2">
                <span 
                  class="text-[9px] font-black uppercase tracking-wider px-2 py-0.5 rounded-full"
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
                  @click.stop="removeTask(index)" 
                  class="text-app-text-mute hover:text-app-rose p-1 rounded-lg hover:bg-app-bg transition-colors"
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
                    @click.stop="handleSelectTargetDir(task)"
                    class="px-2 py-1.5 border border-app-border hover:bg-app-bg text-[10px] font-black rounded-lg text-app-text-dim transition-colors shrink-0"
                    :disabled="isProcessing"
                  >
                    更改
                  </button>
                </div>
              </div>

              <!-- Password settings -->
              <div class="flex flex-col gap-1">
                <span class="font-black text-app-text-dim flex items-center gap-1">
                  <span>专属解压密码</span>
                  <span class="text-[9px] font-medium opacity-50">(可不填，多个用逗号隔开)</span>
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
        </div>

      </div>

      <!-- Right Panel: Side Log Panel (if not wide enough, can be toggled) -->
      <div class="w-80 flex flex-col bg-app-surface border border-app-border rounded-[28px] shadow-app-md overflow-hidden shrink-0">
        <div class="px-6 py-4 border-b border-app-border bg-app-surface flex items-center justify-between shrink-0">
          <span class="text-sm font-black text-app-text">当前任务日志</span>
          <span class="text-[10px] font-bold text-app-text-mute font-mono">DEBUG LOG</span>
        </div>
        
        <!-- Scrollable specific logs -->
        <div class="flex-1 p-6 overflow-y-auto space-y-2.5 custom-scrollbar bg-slate-950 text-slate-100 font-mono text-[10px] leading-relaxed select-text">
          <div v-if="activeLogTask">
            <div class="text-slate-400 font-bold border-b border-slate-800 pb-2 mb-3">
              === 任务: {{ activeLogTask.name }} ===
            </div>
            <div v-if="activeLogTask.log.length === 0" class="text-slate-500 italic">
              等待任务开始...
            </div>
            <div 
              v-for="(msg, i) in activeLogTask.log" 
              :key="i"
              class="break-all border-l-2 pl-2 border-slate-700 hover:bg-slate-900/50 py-0.5 transition-colors"
            >
              {{ msg }}
            </div>
          </div>
          <div v-else class="h-full flex items-center justify-center text-slate-500 italic text-center p-4">
            在左侧点击任务卡片<br />可实时查看单项解压日志
          </div>
        </div>
      </div>

    </div>

    <!-- Bottom Collapsible Global Console Terminal -->
    <div 
      class="border-t border-app-border bg-app-surface/90 backdrop-blur-md flex flex-col transition-all duration-500 shrink-0"
      :style="{ height: isConsoleOpen ? '220px' : '44px' }"
    >
      <!-- Console Header Toggle -->
      <div 
        @click="isConsoleOpen = !isConsoleOpen" 
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
          <span class="text-[10px] text-app-text-mute font-medium">点击{{ isConsoleOpen ? '折叠' : '展开' }}</span>
          <div class="transition-transform duration-300" :class="{ 'rotate-180': isConsoleOpen }">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7" />
            </svg>
          </div>
        </div>
      </div>

      <!-- Console Body Logs -->
      <div 
        id="console-logs-container"
        class="flex-1 p-6 overflow-y-auto space-y-1.5 custom-scrollbar bg-slate-950 font-mono text-[10px] text-slate-300 leading-relaxed select-text"
        v-show="isConsoleOpen"
      >
        <div v-if="consoleLogs.length === 0" class="text-slate-500 italic">
          系统控制台就绪，当前暂无操作日志。
        </div>
        <div 
          v-for="(log, i) in consoleLogs" 
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

    <!-- Settings Modal Dynamic Control -->
    <SettingsModal 
      v-model:show="showSettings" 
      v-model:settings="appSettings"
      :groups="settingGroups"
      @save="saveSettings"
    />

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
</style>