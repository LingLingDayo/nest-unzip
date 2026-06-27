import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { SettingGroup } from "../components/common/SettingsModal/types";
import { extractDefaultSettings } from "../components/common/SettingsModal/utils";

export const settingGroups: SettingGroup[] = [
  {
    id: "tools",
    title: "解压引擎配置",
    description: "配置 7-Zip 或 Bandizip 的目录及首选解压工具。",
    color: "#0ea5e9",
    items: [
      {
        id: "sevenZipDir",
        type: "input",
        label: "7-Zip 安装目录",
        description: "通常位于 C:\\Program Files\\7-Zip，为空则默认自动检测",
        defaultValue: "",
      },
      {
        id: "bandizipDir",
        type: "input",
        label: "Bandizip 安装目录",
        description: "通常位于 C:\\Program Files\\Bandizip，为空则默认自动检测",
        defaultValue: "",
      },
      {
        id: "preferredTool",
        type: "select",
        label: "首选解压引擎",
        description: "系统将优先使用该工具执行解压缩任务。",
        defaultValue: "7z",
        options: [
          { label: "7-Zip", value: "7z" },
          { label: "Bandizip", value: "bandizip" },
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

function getDirectoryOfPath(filePath: string): string {
  if (!filePath) return "";
  let s = filePath.trim();
  if (s.startsWith('"') && s.endsWith('"') && s.length >= 2) {
    s = s.slice(1, -1);
  }
  s = s.trim();
  if (!s.includes("/") && !s.includes("\\")) {
    return "";
  }
  const normalized = s.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash > -1) {
    return s.substring(0, lastSlash);
  }
  return s;
}

export function useSettings(addLog: (taskName: string, message: string, type?: "info" | "success" | "error") => void) {
  const appSettings = reactive(extractDefaultSettings(settingGroups));

  const detectedToolsState = reactive({
    sevenZip: false,
    bandizip: false,
  });

  const loadSettings = async () => {
    const saved = localStorage.getItem("unzip_nest_settings");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        if (parsed.sevenZipPath && !parsed.sevenZipDir) {
          parsed.sevenZipDir = getDirectoryOfPath(parsed.sevenZipPath);
        }
        if (parsed.bandizipPath && !parsed.bandizipDir) {
          parsed.bandizipDir = getDirectoryOfPath(parsed.bandizipPath);
        }
        delete parsed.sevenZipPath;
        delete parsed.bandizipPath;
        Object.assign(appSettings, parsed);
      } catch (e) {
        console.error(e);
      }
    }

    try {
      const detected: any = await invoke("detect_tools");
      detectedToolsState.sevenZip = !!detected.seven_zip;
      detectedToolsState.bandizip = !!detected.bandizip;

      if (!appSettings.sevenZipDir && detected.seven_zip) {
        appSettings.sevenZipDir = getDirectoryOfPath(detected.seven_zip);
      }
      if (!appSettings.bandizipDir && detected.bandizip) {
        appSettings.bandizipDir = getDirectoryOfPath(detected.bandizip);
      }

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

  return {
    appSettings,
    detectedToolsState,
    loadSettings,
    saveSettings,
  };
}
