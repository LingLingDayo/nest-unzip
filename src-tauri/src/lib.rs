use std::path::Path;
use std::process::Command;
use tauri::Emitter;

#[derive(serde::Serialize)]
struct DetectedTools {
    seven_zip: Option<String>,
    bandizip: Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
    task_id: String,
    message: String,
    status: String, // "running", "success", "error"
    progress: f32,
}

#[cfg(target_os = "windows")]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

#[cfg(not(target_os = "windows"))]
fn hide_window(_cmd: &mut Command) {}

fn is_command_available(cmd: &str) -> bool {
    let check_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    Command::new(check_cmd)
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn clean_path(path: &str) -> String {
    let mut s = path.trim().to_string();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s.remove(0);
        s.pop();
    }
    s.trim().to_string()
}

fn resolve_exe_path(dir_or_path: &str, exe_type: &str) -> Option<String> {
    let cleaned = clean_path(dir_or_path);
    
    // 如果为空，执行自动检测
    if cleaned.is_empty() {
        if exe_type == "7z" {
            if is_command_available("7z") {
                return Some("7z".to_string());
            }
            let paths = [
                "C:\\Program Files\\7-Zip\\7z.exe",
                "C:\\Program Files (x86)\\7-Zip\\7z.exe",
            ];
            for p in &paths {
                if Path::new(p).exists() {
                    return Some(p.to_string());
                }
            }
        } else {
            if is_command_available("bc") {
                return Some("bc".to_string());
            }
            let paths = [
                "C:\\Program Files\\Bandizip\\bc.exe",
                "C:\\Program Files\\Bandizip\\Bandizip.exe",
                "C:\\Program Files (x86)\\Bandizip\\bc.exe",
            ];
            for p in &paths {
                if Path::new(p).exists() {
                    return Some(p.to_string());
                }
            }
        }
        return None;
    }
    
    let path = Path::new(&cleaned);
    
    // 1. 如果本身是一个存在的文件，直接返回
    if path.is_file() && path.exists() {
        return Some(cleaned);
    }
    
    // 2. 如果是目录，在里面寻找对应的 exe
    let exe_names = if exe_type == "7z" {
        vec!["7z.exe", "7Z.exe"]
    } else {
        vec!["bc.exe", "Bandizip.exe", "BC.exe", "BANDIZIP.exe"]
    };
    
    for name in &exe_names {
        let exe_path = path.join(name);
        if exe_path.exists() {
            return Some(exe_path.to_string_lossy().to_string());
        }
    }
    
    // 3. 兜底：如果不是目录但可能是个去掉 .exe 结尾的路径，尝试拼接寻找
    if !cleaned.to_lowercase().ends_with(".exe") {
        for name in &exe_names {
            let exe_path = path.join(name);
            if exe_path.exists() {
                return Some(exe_path.to_string_lossy().to_string());
            }
        }
    }
    
    // 4. 否则直接返回清理后的路径
    Some(cleaned)
}

#[tauri::command]
fn detect_tools() -> Result<DetectedTools, String> {
    let mut seven_zip = None;
    let mut bandizip = None;

    // 1. 检查环境变量
    if is_command_available("7z") {
        seven_zip = Some("7z".to_string());
    }
    if is_command_available("bc") {
        bandizip = Some("bc".to_string());
    }

    // 2. 检查常见路径
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

fn try_extract_7z(
    exe_path: &str,
    archive: &str,
    out_dir: &str,
    password: Option<&str>,
) -> Result<(), String> {
    let mut cmd = Command::new(exe_path);
    cmd.arg("x")
        .arg(archive)
        .arg(format!("-o{}", out_dir))
        .arg("-y");
    if let Some(p) = password {
        cmd.arg(format!("-p{}", p));
    }
    hide_window(&mut cmd);

    let output = cmd.output().map_err(|e| format!("执行 7z 失败: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let err_text = String::from_utf8_lossy(&output.stderr).to_string()
            + "\n"
            + &String::from_utf8_lossy(&output.stdout).to_string();
        Err(err_text)
    }
}

fn try_extract_bc(
    exe_path: &str,
    archive: &str,
    out_dir: &str,
    password: Option<&str>,
) -> Result<(), String> {
    let mut cmd = Command::new(exe_path);
    cmd.arg("x")
        .arg(format!("-o:{}", out_dir))
        .arg("-y")
        .arg(archive);
    if let Some(p) = password {
        cmd.arg(format!("-p:{}", p));
    }
    hide_window(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("执行 Bandizip 失败: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let err_text = String::from_utf8_lossy(&output.stderr).to_string()
            + "\n"
            + &String::from_utf8_lossy(&output.stdout).to_string();
        Err(err_text)
    }
}

fn extract_single_archive(
    exe_path: &str,
    exe_type: &str,
    archive_path: &str,
    target_dir: &str,
    passwords: &[String],
) -> Result<(), String> {
    // 1. 尝试无密码
    let res = if exe_type == "7z" {
        try_extract_7z(exe_path, archive_path, target_dir, None)
    } else {
        try_extract_bc(exe_path, archive_path, target_dir, None)
    };

    if res.is_ok() {
        return Ok(());
    }

    let mut last_err = res.unwrap_err();

    // 2. 依次尝试提供的密码
    for pwd in passwords {
        if pwd.trim().is_empty() {
            continue;
        }
        let res = if exe_type == "7z" {
            try_extract_7z(exe_path, archive_path, target_dir, Some(pwd))
        } else {
            try_extract_bc(exe_path, archive_path, target_dir, Some(pwd))
        };
        if res.is_ok() {
            return Ok(());
        }
        last_err = res.unwrap_err();
    }

    Err(format!("密码错误或文件损坏: {}", last_err))
}

fn find_archives_in_dir(dir_path: &str) -> Result<Vec<String>, String> {
    let mut archives = Vec::new();
    let extensions = ["zip", "7z", "rar", "tar", "gz", "bz2", "xz"];

    fn visit_dirs(
        dir: &std::path::Path,
        archives: &mut Vec<String>,
        extensions: &[&str],
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, archives, extensions)?;
                } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if extensions.contains(&ext_lower.as_str()) {
                        if let Some(path_str) = path.to_str() {
                            archives.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    let path = std::path::Path::new(dir_path);
    if path.exists() {
        visit_dirs(path, &mut archives, &extensions).map_err(|e| format!("遍历目录失败: {}", e))?;
    }

    Ok(archives)
}

fn run_extraction_flow(
    app_handle: tauri::AppHandle,
    task_id: String,
    archive_path: String,
    target_dir: String,
    passwords: Vec<String>,
    exe_path: String,
    exe_type: String,
) -> Result<(), String> {
    let resolved_exe_path = resolve_exe_path(&exe_path, &exe_type).ok_or_else(|| {
        format!("无法在配置的路径中找到 {} 的可执行文件", exe_type)
    })?;

    let emit_log = |msg: &str, status: &str, progress: f32| {
        let _ = app_handle.emit(
            "extract-log",
            LogPayload {
                task_id: task_id.clone(),
                message: msg.to_string(),
                status: status.to_string(),
                progress,
            },
        );
    };

    let dir_existed_before = std::path::Path::new(&target_dir).exists();

    // 错误时的清理闭包
    let cleanup_on_error = || {
        emit_log("解压出错，正在清理中间产物到回收站...", "running", 95.0);
        if !dir_existed_before {
            let path = std::path::Path::new(&target_dir);
            if path.exists() {
                let _ = trash::delete(path);
            }
        } else {
            if let Ok(nested_archives) = find_archives_in_dir(&target_dir) {
                for archive in nested_archives {
                    let _ = trash::delete(std::path::Path::new(&archive));
                }
            }
        }
    };

    emit_log("开始第一层解压...", "running", 10.0);

    // 1. 第一层解压
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        let err_msg = format!("创建目标文件夹失败: {}", e);
        emit_log(&err_msg, "error", 100.0);
        err_msg
    })?;

    if let Err(e) =
        extract_single_archive(&resolved_exe_path, &exe_type, &archive_path, &target_dir, &passwords)
    {
        let err_msg = format!("第一层解压失败: {}", e);
        emit_log(&err_msg, "error", 100.0);
        cleanup_on_error();
        return Err(err_msg);
    }

    emit_log("第一层解包成功，开始扫描嵌套压缩包...", "running", 35.0);

    // 2. 扫描并深度解压
    let max_depth = 20;
    let mut depth = 1;
    loop {
        if depth > max_depth {
            let err_msg = "达到最大嵌套解包深度限制 (20层)，停止解包。".to_string();
            emit_log(&err_msg, "error", 100.0);
            cleanup_on_error();
            return Err(err_msg);
        }

        let nested_archives = match find_archives_in_dir(&target_dir) {
            Ok(archives) => archives,
            Err(e) => {
                let err_msg = format!("扫描嵌套包失败: {}", e);
                emit_log(&err_msg, "error", 100.0);
                cleanup_on_error();
                return Err(err_msg);
            }
        };

        if nested_archives.is_empty() {
            break;
        }

        emit_log(
            &format!(
                "第 {} 层扫描找到 {} 个嵌套压缩包...",
                depth,
                nested_archives.len()
            ),
            "running",
            35.0 + (depth as f32 * 5.0).min(45.0),
        );

        for sub_archive in nested_archives {
            let filename = std::path::Path::new(&sub_archive)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("未知压缩包")
                .to_string();

            let parent_dir = match std::path::Path::new(&sub_archive).parent() {
                Some(p) => p.to_str().unwrap_or(&target_dir).to_string(),
                None => target_dir.clone(),
            };

            emit_log(
                &format!("正在解压嵌套子包: {}", filename),
                "running",
                35.0 + (depth as f32 * 5.0).min(45.0),
            );

            if let Err(e) =
                extract_single_archive(&resolved_exe_path, &exe_type, &sub_archive, &parent_dir, &passwords)
            {
                let err_msg = format!("解压嵌套子包 {} 失败: {}", filename, e);
                emit_log(&err_msg, "error", 100.0);
                cleanup_on_error();
                return Err(err_msg);
            }

            emit_log(
                &format!("嵌套子包 {} 解包成功，移动中间包到回收站。", filename),
                "running",
                35.0 + (depth as f32 * 5.0).min(45.0),
            );

            if let Err(e) = trash::delete(std::path::Path::new(&sub_archive)) {
                emit_log(
                    &format!("移入回收站失败: {}, 尝试物理删除...", e),
                    "running",
                    35.0 + (depth as f32 * 5.0).min(45.0),
                );
                let _ = std::fs::remove_file(&sub_archive);
            }
        }

        depth += 1;
    }

    emit_log(
        "全部深度解压完成，已成功清理所有中间压缩包！",
        "success",
        100.0,
    );
    Ok(())
}

#[derive(serde::Serialize)]
struct ExtractResult {
    success: bool,
    #[serde(rename = "errorType")]
    error_type: String, // "None" | "PasswordRequired" | "Other"
    message: String,
}

#[tauri::command]
fn extract_archive(
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

    match extract_single_archive(&resolved_exe_path, &exe_type, &archive_path, &target_dir, &passwords) {
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
                || err_lower.contains("encrypted");

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
fn scan_archives(dir_path: String) -> Result<Vec<String>, String> {
    find_archives_in_dir(&dir_path)
}

#[tauri::command]
fn trash_path(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        trash::delete(p).map_err(|e| format!("移入回收站失败: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
fn scan_dir_entries(dir_path: String) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    let path = std::path::Path::new(&dir_path);
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
fn run_depth_extraction(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            detect_tools,
            run_depth_extraction,
            extract_archive,
            scan_archives,
            trash_path,
            path_exists,
            scan_dir_entries
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
