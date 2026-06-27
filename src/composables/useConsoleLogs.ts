import { ref } from "vue";

export interface ConsoleLog {
  time: string;
  taskName: string;
  message: string;
  type: "info" | "success" | "error";
}

export function useConsoleLogs() {
  const consoleLogs = ref<ConsoleLog[]>([]);
  const isConsoleOpen = ref(false);

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

  const clearLogs = () => {
    consoleLogs.value = [];
  };

  return {
    consoleLogs,
    isConsoleOpen,
    addLog,
    clearLogs,
  };
}
