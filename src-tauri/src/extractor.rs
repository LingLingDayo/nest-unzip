use std::io::{BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

use crate::types::LogPayload;
use crate::utils::{clean_path, hide_window, is_command_available};

#[cfg(target_os = "windows")]
use crate::utils::query_registry;

pub fn resolve_exe_path(dir_or_path: &str, exe_type: &str) -> Option<String> {
    let cleaned = clean_path(dir_or_path);

    // 如果为空，执行自动检测
    if cleaned.is_empty() {
        if exe_type == "7z" {
            if is_command_available("7z") {
                return Some("7z".to_string());
            }
            #[cfg(target_os = "windows")]
            {
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
                            return Some(exe_path.to_string_lossy().to_string());
                        }
                    }
                }
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
            #[cfg(target_os = "windows")]
            {
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
                            return Some(bc_path.to_string_lossy().to_string());
                        }
                        let bz_path = base_path.join("Bandizip.exe");
                        if bz_path.exists() {
                            return Some(bz_path.to_string_lossy().to_string());
                        }
                    }
                }
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

fn parse_7z_percent(s: &str) -> Option<f32> {
    if let Some(pct_idx) = s.find('%') {
        let before = &s[..pct_idx];
        let mut num_str = String::new();
        for c in before.chars().rev() {
            if c.is_ascii_digit() {
                num_str.insert(0, c);
            } else if c == ' ' {
                if !num_str.is_empty() {
                    break;
                }
            } else {
                break;
            }
        }
        if !num_str.is_empty() {
            return num_str.parse::<f32>().ok();
        }
    }
    None
}

fn try_extract_7z(
    exe_path: &str,
    archive: &str,
    out_dir: &str,
    password: Option<&str>,
    progress_callback: Option<&dyn Fn(f32, &str)>,
) -> Result<(), String> {
    let mut cmd = Command::new(exe_path);
    cmd.arg("x")
        .arg(archive)
        .arg(format!("-o{}", out_dir))
        .arg("-y")
        .arg("-bsp1"); // 强制 7z 输出进度到 stdout
    if let Some(p) = password {
        cmd.arg(format!("-p{}", p));
    }
    hide_window(&mut cmd);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("启动 7z 失败: {}", e))?;

    let stdout = child.stdout.take().ok_or("无法获取 stdout 管道")?;
    let stderr = child.stderr.take().ok_or("无法获取 stderr 管道")?;

    let stderr_output = Arc::new(Mutex::new(String::new()));
    let stderr_output_clone = stderr_output.clone();

    let stderr_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        if let Ok(_) = reader.read_to_string(&mut buf) {
            if let Ok(mut guard) = stderr_output_clone.lock() {
                *guard = buf;
            }
        }
    });

    let mut stdout_output = String::new();
    {
        let mut reader = BufReader::new(stdout);
        let mut buf = [0u8; 1024];
        let mut current_line = Vec::new();

        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            for &byte in &buf[..n] {
                if byte == b'\r' || byte == b'\n' {
                    if !current_line.is_empty() {
                        let line_str = String::from_utf8_lossy(&current_line);
                        stdout_output.push_str(&line_str);
                        stdout_output.push('\n');
                        if let Some(pct) = parse_7z_percent(&line_str) {
                            if let Some(cb) = progress_callback {
                                cb(pct, &line_str);
                            }
                        }
                        current_line.clear();
                    }
                } else {
                    current_line.push(byte);
                }
            }
        }
        if !current_line.is_empty() {
            let line_str = String::from_utf8_lossy(&current_line);
            stdout_output.push_str(&line_str);
            if let Some(pct) = parse_7z_percent(&line_str) {
                if let Some(cb) = progress_callback {
                    cb(pct, &line_str);
                }
            }
        }
    }

    let _ = stderr_thread.join();

    let status = child
        .wait()
        .map_err(|e| format!("等待 7z 结束失败: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        let err_guard = stderr_output.lock().unwrap();
        let err_text = format!("{}\n{}", *err_guard, stdout_output);
        Err(err_text)
    }
}

fn try_extract_bc(
    exe_path: &str,
    archive: &str,
    out_dir: &str,
    password: Option<&str>,
    _progress_callback: Option<&dyn Fn(f32, &str)>,
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

pub fn extract_single_archive(
    exe_path: &str,
    exe_type: &str,
    archive_path: &str,
    target_dir: &str,
    passwords: &[String],
    progress_callback: Option<&dyn Fn(f32, &str)>,
) -> Result<(), String> {
    // 1. 尝试无密码
    let res = if exe_type == "7z" {
        try_extract_7z(exe_path, archive_path, target_dir, None, progress_callback)
    } else {
        try_extract_bc(exe_path, archive_path, target_dir, None, progress_callback)
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
            try_extract_7z(
                exe_path,
                archive_path,
                target_dir,
                Some(pwd),
                progress_callback,
            )
        } else {
            try_extract_bc(
                exe_path,
                archive_path,
                target_dir,
                Some(pwd),
                progress_callback,
            )
        };
        if res.is_ok() {
            return Ok(());
        }
        last_err = res.unwrap_err();
    }

    Err(format!("密码错误或文件损坏: {}", last_err))
}

pub fn find_archives_in_dir(dir_path: &str) -> Result<Vec<String>, String> {
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

pub fn run_extraction_flow(
    app_handle: tauri::AppHandle,
    task_id: String,
    archive_path: String,
    target_dir: String,
    passwords: Vec<String>,
    exe_path: String,
    exe_type: String,
) -> Result<(), String> {
    let resolved_exe_path = resolve_exe_path(&exe_path, &exe_type)
        .ok_or_else(|| format!("无法在配置的路径中找到 {} 的可执行文件", exe_type))?;

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

    let emit_log_ref = &emit_log;
    if let Err(e) = extract_single_archive(
        &resolved_exe_path,
        &exe_type,
        &archive_path,
        &target_dir,
        &passwords,
        Some(&|pct, _| {
            emit_log_ref(
                &format!("开始第一层解压 ({}%)...", pct as i32),
                "running",
                10.0 + pct * 0.25, // 10.0 -> 35.0
            );
        }),
    ) {
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

        let nested_count = nested_archives.len();
        for (i, sub_archive) in nested_archives.into_iter().enumerate() {
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
                &format!("正在解压嵌套子包: {} (0%)...", filename),
                "running",
                35.0 + ((depth - 1) as f32 * 5.0).min(45.0),
            );

            let emit_log_ref = &emit_log;
            let filename_ref = &filename;
            if let Err(e) = extract_single_archive(
                &resolved_exe_path,
                &exe_type,
                &sub_archive,
                &parent_dir,
                &passwords,
                Some(&|pct, _| {
                    let current_layer_progress = (i as f32 + pct / 100.0) / nested_count as f32;
                    let start_progress = 35.0 + ((depth - 1) as f32 * 5.0).min(45.0);
                    let next_progress = 35.0 + (depth as f32 * 5.0).min(45.0);
                    let step = next_progress - start_progress;
                    let progress = start_progress + current_layer_progress * step;
                    emit_log_ref(
                        &format!("正在解压嵌套子包: {} ({}%)...", filename_ref, pct as i32),
                        "running",
                        progress,
                    );
                }),
            ) {
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

#[cfg(test)]
mod tests {
    use super::parse_7z_percent;

    #[test]
    fn test_parse_7z_percent() {
        assert_eq!(parse_7z_percent("  0%"), Some(0.0));
        assert_eq!(parse_7z_percent("  3%"), Some(3.0));
        assert_eq!(parse_7z_percent(" 12% 435"), Some(12.0));
        assert_eq!(parse_7z_percent("100%"), Some(100.0));
        assert_eq!(
            parse_7z_percent("Extracting  archive.zip   15%"),
            Some(15.0)
        );
        assert_eq!(parse_7z_percent("Everything is Ok"), None);
        assert_eq!(parse_7z_percent("  %"), None);
    }
}
