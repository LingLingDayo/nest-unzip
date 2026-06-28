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
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout_str.lines().rev() {
        let found_idx = line.find(" files,")
            .or_else(|| line.find(" files"))
            .or_else(|| line.find(" file(s)"))
            .or_else(|| line.find(" 个文件"));

        if let Some(idx) = found_idx {
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

    let mut cmd = Command::new(exe_path);
    cmd.arg("x").arg(format!("-o:{}", out_dir)).arg("-y");
    if let Some(p) = password {
        cmd.arg(format!("-p:{}", p));
    }
    cmd.arg(archive);
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
                                || trimmed.starts_with("bc ")
                                || trimmed.starts_with("ERROR")
                                || trimmed.starts_with("Error")
                                || trimmed.contains("Password required")
                                || trimmed.starts_with("----")
                                || trimmed.starts_with("Date  Time")
                                || trimmed.starts_with("Listing archive:")
                                || trimmed.starts_with("Archive format:");
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
                    || trimmed.starts_with("bc ")
                    || trimmed.starts_with("ERROR")
                    || trimmed.starts_with("Error")
                    || trimmed.contains("Password required")
                    || trimmed.starts_with("----")
                    || trimmed.starts_with("Date  Time")
                    || trimmed.starts_with("Listing archive:")
                    || trimmed.starts_with("Archive format:");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_bandizip_extraction() {
        let exe_path = "H:\\APP\\Bandizip\\bz.exe";
        if !Path::new(exe_path).exists() {
            println!("bz.exe 不存在，跳过测试");
            return;
        }

        let archive = "H:\\Projects\\aibuild\\nest-unzip\\.temp\\nopwd.zip";
        let out_dir = "H:\\Projects\\aibuild\\nest-unzip\\.temp\\out_test_extract";

        let total = get_bandizip_total_files(exe_path, archive, None);
        println!("总文件数: {:?}", total);

        let res = try_extract_bc(exe_path, archive, out_dir, None, Some(&|pct, file| {
            println!("进度: {:.1}%, 发生于文件: {}", pct, file);
        }));

        println!("结果: {:?}", res);
        assert!(res.is_ok());
    }
}
