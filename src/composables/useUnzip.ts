import { ref, computed, onMounted, onUnmounted, type Ref } from "vue";
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
  selected?: boolean; // 是否选中
  currentDepth?: number; // 当前解压层级
  currentIndex?: number; // 当前层级内的索引
  totalInLevel?: number; // 当前层级的总文件数
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

  const finalizeExtraction = async (task: ExtractTask, logSuccessDetail: string) => {
    if (appSettings.flattenSingleSubdir) {
      addLog(task.name, "正在检查并整理单子目录...", "info");
      try {
        await invoke("flatten_single_subdir", { dirPath: task.targetDir });
        addLog(task.name, "单子目录提升整理完成！", "info");
      } catch (err) {
        addLog(task.name, `整理单子目录时出错: ${err}`, "error");
      }
    }

    task.status = "success";
    task.progress = 100;
    addLog(task.name, logSuccessDetail, "success");

    if (appSettings.autoOpen) {
      openPath(task.targetDir).catch((err) => {
        addLog(task.name, `自动打开目标文件夹失败: ${err}`, "error");
      });
    }
  };

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
          selected: false,
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

    const selectedTasks = tasks.value.filter((t) => t.selected);
    const tasksToProcess = selectedTasks.length > 0 ? selectedTasks : tasks.value;

    const pendingTasks = tasksToProcess.filter((t) => t.status !== "success");
    if (pendingTasks.length === 0) {
      addLog("系统", "所选任务已全部解压成功，无需重复执行");
      return;
    }

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

    for (let i = 0; i < tasksToProcess.length; i++) {
      const task = tasksToProcess[i];
      if (task.status === "success") continue;

      task.status = "running";
      task.progress = 0;

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

      addLog(task.name, `解压缩流程初始化... (文件路径: ${task.path}, 目标目录: ${task.targetDir})`);
      addLog(task.name, `合并密码池完成，共 ${mergedPasswords.length} 个备选密码已就绪`);

      let dirExistedBefore = false;
      let initialEntries: string[] = [];
      try {
        dirExistedBefore = await invoke<boolean>("path_exists", { path: task.targetDir });
        if (dirExistedBefore) {
          initialEntries = await invoke<string[]>("scan_dir_entries", { dirPath: task.targetDir });
          addLog(task.name, `目标目录已存在，扫描到 ${initialEntries.length} 个已有文件，解压后将保留它们。`, "info");
        } else {
          addLog(task.name, `目标目录不存在，将在解压时自动创建: ${task.targetDir}`, "info");
        }
      } catch (e) {
        console.error(e);
        addLog(task.name, `检查目标目录出错: ${e}`, "error");
      }

      const cleanupOnError = async () => {
        const deletePermanently = !!appSettings.deletePermanently;
        const actionName = deletePermanently ? "正在完全删除中间产物" : "正在清理中间产物到回收站";
        const command = deletePermanently ? "delete_path" : "trash_path";

        addLog(task.name, `${actionName} (目标目录: ${task.targetDir})...`, "info");
        try {
          if (!dirExistedBefore) {
            await invoke(command, { path: task.targetDir });
            addLog(task.name, `已清理创建的目标文件夹: ${task.targetDir}`, "info");
          } else {
            const currentEntries = await invoke<string[]>("scan_dir_entries", { dirPath: task.targetDir });
            const addedEntries = currentEntries.filter(item => !initialEntries.includes(item));
            addLog(task.name, `扫描到 ${addedEntries.length} 个新增的临时中间文件，准备清理...`, "info");
            for (const entry of addedEntries) {
              await invoke(command, { path: entry });
              addLog(task.name, `已成功清理临时产物: ${entry.split(/[\\/]/).pop() || entry}`, "info");
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

        addLog(task.name, `开始第一层解包流程 (源文件: ${task.name})...`, "info");

        while (queue.length > 0) {
          if (depth > maxDepth) {
            throw new Error("达到最大嵌套解包深度限制 (20层)，停止解包。");
          }

          const currentLevelArchives = [...queue];
          queue = [];

          addLog(task.name, `第 ${depth} 层：开始解包 ${currentLevelArchives.length} 个文件...`, "info");
          for (const item of currentLevelArchives) {
            addLog(task.name, ` -> 待解压队列成员: ${item.split(/[\\/]/).pop() || item}`, "info");
          }

          const nestedCount = currentLevelArchives.length;
          for (let index = 0; index < nestedCount; index++) {
            const subArchive = currentLevelArchives[index];
            const filename = subArchive.split(/[\\/]/).pop() || "未知压缩包";
            
            // 保存当前层级元数据，供进度计算监听器使用
            task.currentDepth = depth;
            task.currentIndex = index;
            task.totalInLevel = nestedCount;

            let currentTargetDir = task.targetDir;
            if (subArchive !== task.path) {
              const lastSlash = Math.max(subArchive.lastIndexOf("/"), subArchive.lastIndexOf("\\"));
              currentTargetDir = lastSlash > -1 ? subArchive.substring(0, lastSlash) : task.targetDir;
            }
            
            let isExtracted = false;
            while (!isExtracted) {
              addLog(task.name, `尝试解压: ${filename} -> ${currentTargetDir} (已配置密码池包含 ${mergedPasswords.length} 个密码)`, "info");
              const result = await invoke<{ success: boolean; errorType: string; message: string }>("extract_archive", {
                taskId: task.id,
                exePath: exePath,
                exeType: exeType,
                archivePath: subArchive,
                targetDir: currentTargetDir,
                passwords: mergedPasswords,
              });

              if (result.success) {
                isExtracted = true;
                addLog(task.name, `成功解压文件: ${filename}`, "success");
                if (subArchive !== task.path) {
                  const command = appSettings.deletePermanently ? "delete_path" : "trash_path";
                  await invoke(command, { path: subArchive });
                  addLog(task.name, `已清理临时源文件: ${filename}`, "info");
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
                    addLog(task.name, `已将本次手动输入成功的密码保存至该任务的专属密码中`, "info");
                  }
                  lastInputPwd = null; // 重置本轮输入记录
                }
              } else {
                if (result.errorType === "PasswordRequired") {
                  addLog(task.name, `解压失败: 文件 [${filename}] 密码不正确或缺失，当前密码池已穷尽。`, "error");
                  if (!passwordModalRef.value) {
                    throw new Error("密码弹窗组件未挂载，解包终止");
                  }

                  addLog(task.name, `正在呼叫用户输入密码...`, "info");
                  const newPassword = await passwordModalRef.value.open({
                    title: "输入解压密码",
                    message: `文件 [${filename}] 已加密，请输入正确的解压密码：`,
                    placeholder: "请输入密码",
                  });

                  if (newPassword === null) {
                    addLog(task.name, `用户取消了密码输入，将终止此解压任务`, "error");
                    throw new Error("USER_CANCEL");
                  } else if (newPassword.trim() === "") {
                    addLog(task.name, `输入了空密码，重新请求输入...`, "info");
                    continue;
                  } else {
                    const trimmedPwd = newPassword.trim();
                    addLog(task.name, `收到手动输入的密码，将其加入密码池优先重试`, "info");
                    if (!mergedPasswords.includes(trimmedPwd)) {
                      mergedPasswords = [trimmedPwd, ...mergedPasswords];
                    }
                    lastInputPwd = trimmedPwd; // 暂存本轮输入
                  }
                } else {
                  addLog(task.name, `解压底层引擎报错: ${result.message}`, "error");
                  throw new Error(result.message);
                }
              }
            }
          }

          const found = await invoke<string[]>("scan_archives", { dirPath: task.targetDir });
          if (found && found.length > 0) {
            if (found.length >= 2) {
              const foundNames = found.map(item => item.split(/[\\/]/).pop());
              addLog(task.name, `在当前层发现 ${found.length} 个嵌套压缩包: ${foundNames.join(", ")}，为避免混乱，跳过对这些嵌套包的解压。`, "info");
            } else {
              const foundNames = found.map(item => item.split(/[\\/]/).pop());
              addLog(task.name, `在当前层发现 ${found.length} 个嵌套的下一层压缩包: ${foundNames.join(", ")}，加入待解压队列。`, "info");
              queue.push(...found);
            }
          } else {
            addLog(task.name, `在当前层未发现新的嵌套压缩包`, "info");
          }

          if (queue.length > 0) {
            depth++;
          }
        }

        await finalizeExtraction(task, `解压缩成功！共处理了 ${depth} 层嵌套包，已自动清理所有中间源文件！输出目录: ${task.targetDir}`);


      } catch (e: any) {
        task.status = "error";
        task.progress = 100;
        if (e.message === "USER_CANCEL") {
          addLog(task.name, "解包终止：用户取消了密码输入", "error");
        } else {
          addLog(task.name, `解压缩任务失败: ${e.message || e}，正在执行错误回滚...`, "error");
        }
        await cleanupOnError();
      }
    }

    isProcessing.value = false;
    addLog("系统", "批量深度解压缩全部任务已完成！");
  };

  let unlistenLog: UnlistenFn | null = null;
  let unlistenSingleProgress: UnlistenFn | null = null;
  let unlistenDragDrop: UnlistenFn | null = null;
  let isMounted = true;

  onMounted(async () => {
    isMounted = true;
    try {
      const uLog = await listen("extract-log", (event: any) => {
        const { task_id, message, status, progress } = event.payload;
        const task = tasks.value.find((t) => t.id === task_id);

        if (task) {
          if (status === "success") {
            finalizeExtraction(task, `[成功] ${message}`);
          } else {
            task.progress = progress;
            task.status = status;

            let logType: "info" | "success" | "error" = "info";
            if (status === "error") {
              logType = "error";
              addLog(task.name, `[失败] ${message}`, logType);
            } else {
              addLog(task.name, message, logType);
            }
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
      const uSingleProgress = await listen("single-extract-progress", (event: any) => {
        const { task_id, progress } = event.payload;
        const task = tasks.value.find((t) => t.id === task_id);
        if (task && task.status === "running") {
          const i = task.currentIndex || 0;
          const nestedCount = task.totalInLevel || 1;

          // 计算并显示当前解压层级的整体进度 (0% ~ 100%)
          const currentLayerProgress = ((i + progress / 100.0) / nestedCount) * 100.0;
          task.progress = Math.round(currentLayerProgress);
        }
      });
      if (!isMounted) {
        uSingleProgress();
      } else {
        unlistenSingleProgress = uSingleProgress;
      }
    } catch (e) {
      console.error("注册 single-extract-progress 监听失败:", e);
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
    if (unlistenSingleProgress) unlistenSingleProgress();
    if (unlistenDragDrop) unlistenDragDrop();
  });

  const toggleSelect = (index: number) => {
    if (isProcessing.value) return;
    const task = tasks.value[index];
    if (task) {
      task.selected = !task.selected;
    }
  };

  const selectedCount = computed(() => tasks.value.filter((t) => t.selected).length);
  const isAllSelected = computed(() => tasks.value.length > 0 && tasks.value.every((t) => t.selected));

  const toggleSelectAll = () => {
    if (isProcessing.value) return;
    const allSelected = isAllSelected.value;
    tasks.value.forEach((t) => {
      t.selected = !allSelected;
    });
  };

  return {
    tasks,
    isProcessing,
    addFilesByPaths,
    handleSelectFiles,
    handleSelectTargetDir,
    removeTask,
    clearTasks,
    startBulkExtraction,
    toggleSelect,
    toggleSelectAll,
    selectedCount,
    isAllSelected,
  };
}
