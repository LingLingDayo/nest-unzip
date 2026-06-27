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
    let check_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    Command::new(check_cmd)
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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

    Ok(DetectedTools { seven_zip, bandizip })
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

    let output = cmd.output().map_err(|e| format!("执行 Bandizip 失败: {}", e))?;
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
        visit_dirs(path, &mut archives, &extensions)
            .map_err(|e| format!("遍历目录失败: {}", e))?;
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
) {
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

    emit_log("开始第一层解压...", "running", 10.0);

    // 1. 第一层解压
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        emit_log(&format!("创建目标文件夹失败: {}", e), "error", 100.0);
        return;
    }

    match extract_single_archive(&exe_path, &exe_type, &archive_path, &target_dir, &passwords) {
        Ok(_) => {
            emit_log("第一层解包成功，开始扫描嵌套压缩包...", "running", 35.0);
        }
        Err(e) => {
            emit_log(&format!("第一层解压失败: {}", e), "error", 100.0);
            return;
        }
    }

    // 2. 扫描并深度解压
    let max_depth = 20;
    let mut depth = 1;
    loop {
        if depth > max_depth {
            emit_log("达到最大嵌套解包深度限制 (20层)，停止解包。", "error", 100.0);
            return;
        }

        match find_archives_in_dir(&target_dir) {
            Ok(nested_archives) => {
                if nested_archives.is_empty() {
                    break;
                }

                emit_log(
                    &format!("第 {} 层扫描找到 {} 个嵌套压缩包...", depth, nested_archives.len()),
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

                    match extract_single_archive(&exe_path, &exe_type, &sub_archive, &parent_dir, &passwords) {
                        Ok(_) => {
                            emit_log(
                                &format!("嵌套子包 {} 解包成功，删除中间包。", filename),
                                "running",
                                35.0 + (depth as f32 * 5.0).min(45.0),
                            );
                            let _ = std::fs::remove_file(&sub_archive);
                        }
                        Err(e) => {
                            emit_log(&format!("解压嵌套子包 {} 失败: {}", filename, e), "error", 100.0);
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                emit_log(&format!("扫描嵌套包失败: {}", e), "error", 100.0);
                return;
            }
        }

        depth += 1;
    }

    emit_log("全部深度解压完成，已成功清理所有中间压缩包！", "success", 100.0);
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
) {
    tauri::async_runtime::spawn(async move {
        run_extraction_flow(
            app_handle,
            task_id,
            archive_path,
            target_dir,
            passwords,
            exe_path,
            exe_type,
        );
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![detect_tools, run_depth_extraction])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
