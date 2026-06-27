use crate::extractor::{
    extract_single_archive, find_archives_in_dir, resolve_exe_path, run_extraction_flow,
};
use crate::types::{DetectedTools, ExtractResult};
use crate::utils::is_command_available;
use std::path::Path;

#[cfg(target_os = "windows")]
use crate::utils::query_registry;

#[tauri::command]
pub fn detect_tools() -> Result<DetectedTools, String> {
    let mut seven_zip = None;
    let mut bandizip = None;

    // 1. 检查环境变量
    if is_command_available("7z") {
        seven_zip = Some("7z".to_string());
    }
    if is_command_available("bc") {
        bandizip = Some("bc".to_string());
    }

    // 2. 检查注册表
    #[cfg(target_os = "windows")]
    {
        if seven_zip.is_none() {
            let reg_keys = [
                ("HKLM\\SOFTWARE\\7-Zip", "Path"),
                ("HKLM\\SOFTWARE\\WOW6432Node\\7-Zip", "Path"),
                ("HKCU\\SOFTWARE\\7-Zip", "Path"),
            ];
            for &(key, val_name) in &reg_keys {
                if let Some(path_val) = query_registry(key, val_name) {
                    let base_path = Path::new(&path_val);
                    let exe_path = base_path.join("7z.exe");
                    if exe_path.exists() {
                        seven_zip = Some(exe_path.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }

        if bandizip.is_none() {
            let reg_keys = [
                ("HKLM\\SOFTWARE\\Bandizip", "ProgramFolder"),
                ("HKLM\\SOFTWARE\\WOW6432Node\\Bandizip", "ProgramFolder"),
                ("HKCU\\SOFTWARE\\Bandizip", "ProgramFolder"),
            ];
            for &(key, val_name) in &reg_keys {
                if let Some(path_val) = query_registry(key, val_name) {
                    let base_path = Path::new(&path_val);
                    let bc_path = base_path.join("bc.exe");
                    if bc_path.exists() {
                        bandizip = Some(bc_path.to_string_lossy().to_string());
                        break;
                    }
                    let bz_path = base_path.join("Bandizip.exe");
                    if bz_path.exists() {
                        bandizip = Some(bz_path.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }
    }

    // 3. 检查常见路径
    if seven_zip.is_none() {
        let paths = [
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe",
        ];
        for p in &paths {
            if Path::new(p).exists() {
                seven_zip = Some(p.to_string());
                break;
            }
        }
    }

    if bandizip.is_none() {
        let paths = [
            "C:\\Program Files\\Bandizip\\bc.exe",
            "C:\\Program Files\\Bandizip\\Bandizip.exe",
            "C:\\Program Files (x86)\\Bandizip\\bc.exe",
        ];
        for p in &paths {
            if Path::new(p).exists() {
                bandizip = Some(p.to_string());
                break;
            }
        }
    }

    Ok(DetectedTools {
        seven_zip,
        bandizip,
    })
}

#[tauri::command]
pub fn extract_archive(
    exe_path: String,
    exe_type: String,
    archive_path: String,
    target_dir: String,
    passwords: Vec<String>,
) -> ExtractResult {
    // 确保目标目录存在
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return ExtractResult {
            success: false,
            error_type: "Other".to_string(),
            message: format!("创建目标文件夹失败: {}", e),
        };
    }

    let resolved_exe_path = match resolve_exe_path(&exe_path, &exe_type) {
        Some(path) => path,
        None => {
            return ExtractResult {
                success: false,
                error_type: "Other".to_string(),
                message: format!("无法定位 {} 的可执行文件，请检查路径配置", exe_type),
            };
        }
    };

    match extract_single_archive(
        &resolved_exe_path,
        &exe_type,
        &archive_path,
        &target_dir,
        &passwords,
    ) {
        Ok(_) => ExtractResult {
            success: true,
            error_type: "None".to_string(),
            message: "".to_string(),
        },
        Err(err) => {
            let err_lower = err.to_lowercase();
            let is_pwd_err = err_lower.contains("wrong password")
                || err_lower.contains("password error")
                || err_lower.contains("decryption failed")
                || err_lower.contains("enter password")
                || err_lower.contains("data error in encrypted file")
                || err_lower.contains("can not open encrypted archive")
                || err_lower.contains("encrypted")
                || err_lower.contains("密码")
                || err_lower.contains("加密")
                || err_lower.contains("解密");

            ExtractResult {
                success: false,
                error_type: if is_pwd_err {
                    "PasswordRequired".to_string()
                } else {
                    "Other".to_string()
                },
                message: err,
            }
        }
    }
}

#[tauri::command]
pub fn scan_archives(dir_path: String) -> Result<Vec<String>, String> {
    find_archives_in_dir(&dir_path)
}

#[tauri::command]
pub fn trash_path(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        trash::delete(p).map_err(|e| format!("移入回收站失败: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        if p.is_dir() {
            std::fs::remove_dir_all(p).map_err(|e| format!("物理删除文件夹失败: {}", e))?;
        } else {
            std::fs::remove_file(p).map_err(|e| format!("物理删除文件失败: {}", e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[tauri::command]
pub fn scan_dir_entries(dir_path: String) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    let path = Path::new(&dir_path);
    if path.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Some(path_str) = entry.path().to_str() {
                entries.push(path_str.to_string());
            }
        }
    }
    Ok(entries)
}

#[tauri::command]
pub fn run_depth_extraction(
    app_handle: tauri::AppHandle,
    task_id: String,
    archive_path: String,
    target_dir: String,
    passwords: Vec<String>,
    exe_path: String,
    exe_type: String,
) -> Result<(), String> {
    run_extraction_flow(
        app_handle,
        task_id,
        archive_path,
        target_dir,
        passwords,
        exe_path,
        exe_type,
    )
}
