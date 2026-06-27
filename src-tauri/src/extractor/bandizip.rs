use crate::utils::hide_window;
use std::io::{BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub fn get_bandizip_total_files(
    exe_path: &str,
    archive: &str,
    password: Option<&str>,
) -> Option<usize> {
    let mut cmd = Command::new(exe_path);
    cmd.arg("l");
    if let Some(p) = password {
        cmd.arg(format!("-p:{}", p));
    }
    cmd.arg(archive);
    cmd.stdin(Stdio::null());
    hide_window(&mut cmd);

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines().rev() {
        if let Some(idx) = line.find(" files,") {
            let part = &line[..idx];
            let mut num_str = String::new();
            for c in part.chars().rev() {
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
            if let Ok(count) = num_str.parse::<usize>() {
                return Some(count);
            }
        }
    }
    None
}

pub fn try_extract_bc(
    exe_path: &str,
    archive: &str,
    out_dir: &str,
    password: Option<&str>,
    progress_callback: Option<&dyn Fn(f32, &str)>,
) -> Result<(), String> {
    // 1. 尝试获取压缩包内的文件总数
    let total_files = get_bandizip_total_files(exe_path, archive, password);

    // 2. 构造解压命令
    let mut cmd = Command::new(exe_path);
    cmd.arg("x")
        .arg(format!("-o:{}", out_dir))
        .arg("-y")
        .arg(archive);
    if let Some(p) = password {
        cmd.arg(format!("-p:{}", p));
    }
    cmd.stdin(Stdio::null());
    hide_window(&mut cmd);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 Bandizip 失败: {}", e))?;

    let stdout = child.stdout.take().ok_or("无法获取 stdout 管道")?;
    let stderr = child.stderr.take().ok_or("无法获取 stderr 管道")?;

    let stderr_output = Arc::new(Mutex::new(String::new()));
    let stderr_output_clone = stderr_output.clone();

    let stderr_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        if reader.read_to_string(&mut buf).is_ok() {
            if let Ok(mut guard) = stderr_output_clone.lock() {
                *guard = buf;
            }
        }
    });

    let mut stdout_output = String::new();
    let mut processed_files = 0;

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
                        let trimmed = line_str.trim();
                        if !trimmed.is_empty() {
                            stdout_output.push_str(&line_str);
                            stdout_output.push('\n');

                            // 判断这一行是否代表解压出的文件名
                            let is_info_line = trimmed.starts_with("bz ")
                                || trimmed.starts_with("Extracting archive:")
                                || trimmed.starts_with("bc ");
                            if !is_info_line {
                                processed_files += 1;
                                if let Some(total) = total_files {
                                    if total > 0 {
                                        let pct = (processed_files as f32 / total as f32) * 100.0;
                                        let pct = pct.min(100.0);
                                        if let Some(cb) = progress_callback {
                                            cb(pct, trimmed);
                                        }
                                    }
                                } else if let Some(cb) = progress_callback {
                                    cb(0.0, trimmed);
                                }
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
            let trimmed = line_str.trim();
            if !trimmed.is_empty() {
                stdout_output.push_str(&line_str);
                let is_info_line = trimmed.starts_with("bz ")
                    || trimmed.starts_with("Extracting archive:")
                    || trimmed.starts_with("bc ");
                if !is_info_line {
                    processed_files += 1;
                    if let Some(total) = total_files {
                        if total > 0 {
                            let pct = (processed_files as f32 / total as f32) * 100.0;
                            let pct = pct.min(100.0);
                            if let Some(cb) = progress_callback {
                                cb(pct, trimmed);
                            }
                        }
                    } else if let Some(cb) = progress_callback {
                        cb(0.0, trimmed);
                    }
                }
            }
        }
    }

    let _ = stderr_thread.join();

    let status = child
        .wait()
        .map_err(|e| format!("等待 Bandizip 结束失败: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        let err_guard = stderr_output.lock().unwrap();
        let err_text = format!("{}\n{}", *err_guard, stdout_output);
        Err(err_text)
    }
}
