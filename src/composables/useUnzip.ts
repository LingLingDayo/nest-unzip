import { ref, onMounted, onUnmounted, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

export interface ExtractTask {
  id: string;
  name: string;
  path: string;
  status: "pending" | "running" | "success" | "error";
  progress: number;
  passwords: string; // 专属密码，逗号隔开
  targetDir: string; // 自定义解压输出目录
}

export interface PasswordModalInstance {
  open: (options: { title: string; message: string; placeholder?: string }) => Promise<string | null>;
}

export function useUnzip(
  appSettings: Record<string, any>,
  detectedToolsState: { sevenZip: boolean; bandizip: boolean },
  passwordModalRef: Ref<PasswordModalInstance | null>,
  addLog: (taskName: string, message: string, type?: "info" | "success" | "error") => void
) {
  const tasks = ref<ExtractTask[]>([]);
  const isProcessing = ref(false);

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

  const addFilesByPaths = async (filePaths: string[]) => {
    // 拦截解包中状态，防止并发修改任务队列
    if (isProcessing.value) {
      addLog("系统", "解压任务执行中，暂不支持添加新任务", "error");
      return;
    }

    const extensions = [".zip", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz"];
    let addedCount = 0;

    for (const p of filePaths) {
      const { filename, targetDir } = parsePath(p);
      const isArchive = extensions.some((ext) => filename.toLowerCase().endsWith(ext));
      
      const exists = tasks.value.some((t) => t.path === p);

      if (isArchive && !exists) {
        let finalTargetDir = targetDir;
        let count = 1;
        try {
          while (await invoke<boolean>("path_exists", { path: finalTargetDir })) {
            finalTargetDir = `${targetDir} (${count})`;
            count++;
          }
        } catch (err) {
          console.error("检查文件夹是否存在出错:", err);
        }

        tasks.value.push({
          id: Math.random().toString(36).substring(2, 9),
          name: filename,
          path: p,
          status: "pending",
          progress: 0,
          passwords: "",
          targetDir: finalTargetDir,
        });
        addedCount++;
      }
    }

    if (addedCount > 0) {
      addLog("系统", `成功添加 ${addedCount} 个压缩包文件`);
    }
  };

  const handleSelectFiles = async () => {
    if (isProcessing.value) return;
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
        await addFilesByPaths(selected);
      } else if (selected && typeof selected === "string") {
        await addFilesByPaths([selected]);
      }
    } catch (e) {
      addLog("系统", `选择文件失败: ${e}`, "error");
    }
  };

  const handleSelectTargetDir = async (task: ExtractTask) => {
    if (isProcessing.value) return;
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
    if (isProcessing.value) return;
    const t = tasks.value[index];
    tasks.value.splice(index, 1);
    addLog("系统", `移除了任务: ${t.name}`);
  };

  const clearTasks = () => {
    if (isProcessing.value) return;
    tasks.value = [];
    addLog("系统", "任务列表已清空");
  };

  const startBulkExtraction = async () => {
    if (isProcessing.value || tasks.value.length === 0) return;

    const exeType = appSettings.preferredTool;
    const exePath = exeType === "7z" ? appSettings.sevenZipDir : appSettings.bandizipDir;

    const detectedAvailable = exeType === "7z" ? detectedToolsState.sevenZip : detectedToolsState.bandizip;
    if (!exePath && !detectedAvailable) {
      addLog("系统", `错误: 未配置或未检测到 ${exeType === "7z" ? "7-Zip" : "Bandizip"} 的安装目录。请先在设置中配置！`, "error");
      alert(`请先在右上角设置中配置 ${exeType === "7z" ? "7-Zip" : "Bandizip"} 的安装目录！`);
      return;
    }

    isProcessing.value = true;
    addLog("系统", `批量深度解压启动，引擎: ${exeType === "7z" ? "7-Zip" : "Bandizip"}...`);

    for (let i = 0; i < tasks.value.length; i++) {
      const task = tasks.value[i];
      if (task.status === "success") continue;

      task.status = "running";
      task.progress = 5;

      const globalPwds = ((appSettings.globalPasswords || "") as string)
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      
      const taskPwds = (task.passwords || "")
        .split(" ")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      let mergedPasswords = taskPwds.length > 0
        ? Array.from(new Set(taskPwds))
        : Array.from(new Set(globalPwds));
      let lastInputPwd: string | null = null;

      addLog(task.name, "解压缩流程初始化...");

      let dirExistedBefore = false;
      let initialEntries: string[] = [];
      try {
        dirExistedBefore = await invoke<boolean>("path_exists", { path: task.targetDir });
        if (dirExistedBefore) {
          initialEntries = await invoke<string[]>("scan_dir_entries", { dirPath: task.targetDir });
        }
      } catch (e) {
        console.error(e);
      }

      const cleanupOnError = async () => {
        addLog(task.name, "正在清理中间产物到回收站...", "info");
        try {
          if (!dirExistedBefore) {
            await invoke("trash_path", { path: task.targetDir });
          } else {
            const currentEntries = await invoke<string[]>("scan_dir_entries", { dirPath: task.targetDir });
            const addedEntries = currentEntries.filter(item => !initialEntries.includes(item));
            for (const entry of addedEntries) {
              await invoke("trash_path", { path: entry });
            }
          }
        } catch (err) {
          addLog(task.name, `清理中间产物失败: ${err}`, "error");
        }
      };

      try {
        let queue: string[] = [task.path];
        let depth = 1;
        const maxDepth = 20;

        addLog(task.name, "开始第一层解压...", "info");

        while (queue.length > 0) {
          if (depth > maxDepth) {
            throw new Error("达到最大嵌套解包深度限制 (20层)，停止解包。");
          }

          const currentLevelArchives = [...queue];
          queue = [];

          addLog(task.name, `第 ${depth} 层：开始解压 ${currentLevelArchives.length} 个文件...`, "info");

          for (const subArchive of currentLevelArchives) {
            const filename = subArchive.split(/[\\/]/).pop() || "未知压缩包";
            
            let currentTargetDir = task.targetDir;
            if (subArchive !== task.path) {
              const lastSlash = Math.max(subArchive.lastIndexOf("/"), subArchive.lastIndexOf("\\"));
              currentTargetDir = lastSlash > -1 ? subArchive.substring(0, lastSlash) : task.targetDir;
            }
            
            let isExtracted = false;
            while (!isExtracted) {
              const result = await invoke<{ success: boolean; errorType: string; message: string }>("extract_archive", {
                exePath: exePath,
                exeType: exeType,
                archivePath: subArchive,
                targetDir: currentTargetDir,
                passwords: mergedPasswords,
              });

              if (result.success) {
                isExtracted = true;
                if (subArchive !== task.path) {
                  await invoke("trash_path", { path: subArchive });
                }
                
                // 只有成功解压时，才将最新手动输入的有效密码追加到专属密码框
                if (lastInputPwd) {
                  const currentPwds = (task.passwords || "")
                    .split(" ")
                    .map((s) => s.trim())
                    .filter((s) => s.length > 0);
                  if (!currentPwds.includes(lastInputPwd)) {
                    currentPwds.push(lastInputPwd);
                    task.passwords = currentPwds.join(" ");
                  }
                  lastInputPwd = null; // 重置本轮输入记录
                }
              } else {
                if (result.errorType === "PasswordRequired") {
                  if (!passwordModalRef.value) {
                    throw new Error("密码弹窗组件未挂载，解包终止");
                  }

                  const newPassword = await passwordModalRef.value.open({
                    title: "输入解压密码",
                    message: `文件 [${filename}] 已加密，请输入正确的解压密码：`,
                    placeholder: "请输入密码",
                  });

                  if (newPassword === null) {
                    throw new Error("USER_CANCEL");
                  } else if (newPassword.trim() === "") {
                    continue;
                  } else {
                    const trimmedPwd = newPassword.trim();
                    if (!mergedPasswords.includes(trimmedPwd)) {
                      mergedPasswords = [trimmedPwd, ...mergedPasswords];
                    }
                    lastInputPwd = trimmedPwd; // 暂存本轮输入
                  }
                } else {
                  throw new Error(result.message);
                }
              }
            }
          }

          const found = await invoke<string[]>("scan_archives", { dirPath: task.targetDir });
          if (found && found.length > 0) {
            queue.push(...found);
          }

          const currentProgress = 35 + (depth * 5);
          task.progress = Math.min(currentProgress, 95);

          if (queue.length > 0) {
            depth++;
          }
        }

        task.status = "success";
        task.progress = 100;
        addLog(task.name, "解压缩成功，已成功清理所有中间压缩包！", "success");
        
        if (appSettings.autoOpen) {
          openPath(task.targetDir).catch((err) => {
            addLog(task.name, `自动打开目标文件夹失败: ${err}`, "error");
          });
        }

      } catch (e: any) {
        task.status = "error";
        task.progress = 100;
        if (e.message === "USER_CANCEL") {
          addLog(task.name, "用户取消了解压缩", "error");
        } else {
          addLog(task.name, `任务失败: ${e.message || e}`, "error");
        }
        await cleanupOnError();
      }
    }

    isProcessing.value = false;
    addLog("系统", "批量深度解压缩全部任务已完成！");
  };

  let unlistenLog: UnlistenFn | null = null;
  let unlistenDragDrop: UnlistenFn | null = null;
  let isMounted = true;

  onMounted(async () => {
    isMounted = true;
    try {
      const uLog = await listen("extract-log", (event: any) => {
        const { task_id, message, status, progress } = event.payload;
        const task = tasks.value.find((t) => t.id === task_id);

        if (task) {
          task.progress = progress;
          task.status = status;

          let logType: "info" | "success" | "error" = "info";
          if (status === "success") {
            logType = "success";
            addLog(task.name, `[成功] ${message}`, logType);
            
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

      if (!isMounted) {
        uLog();
      } else {
        unlistenLog = uLog;
      }
    } catch (e) {
      console.error("注册 extract-log 监听失败:", e);
    }

    try {
      const uDrag = await getCurrentWindow().onDragDropEvent(async (event) => {
        if (event.payload.type === "drop") {
          await addFilesByPaths(event.payload.paths);
        }
      });

      if (!isMounted) {
        uDrag();
      } else {
        unlistenDragDrop = uDrag;
      }
    } catch (e) {
      console.error("注册拖拽事件监听失败:", e);
    }
  });

  onUnmounted(() => {
    isMounted = false;
    if (unlistenLog) unlistenLog();
    if (unlistenDragDrop) unlistenDragDrop();
  });

  return {
    tasks,
    isProcessing,
    addFilesByPaths,
    handleSelectFiles,
    handleSelectTargetDir,
    removeTask,
    clearTasks,
    startBulkExtraction,
  };
}
