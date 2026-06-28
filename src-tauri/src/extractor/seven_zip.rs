use crate::utils::hide_window;
use std::io::{BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub fn parse_7z_percent(s: &str) -> Option<f32> {
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

pub fn try_extract_7z(
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
        .arg("-bso1")
        .arg("-bse1")
        .arg("-bsp2"); // 强制进度到 stderr (无缓冲)，错误和输出到 stdout
    if let Some(p) = password {
        cmd.arg(format!("-p{}", p));
    }
    cmd.stdin(Stdio::null());
    hide_window(&mut cmd);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("启动 7z 失败: {}", e))?;

    // 交换：读取实际的 stderr 作为进度流 (主线程)，读取实际的 stdout 作为日志流 (后台线程)
    let stdout = child.stderr.take().ok_or("无法获取 stderr 管道")?;
    let stderr = child.stdout.take().ok_or("无法获取 stdout 管道")?;

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
    {
        let mut reader = BufReader::new(stdout);
        let mut buf = [0u8; 1024];
        let mut current_line = Vec::new();

        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            for &byte in &buf[..n] {
                if byte == b'\r' || byte == b'\n' || byte == b'\x08' {
                    if !current_line.is_empty() {
                        let line_str = String::from_utf8_lossy(&current_line);

                        if byte != b'\x08' {
                            stdout_output.push_str(&line_str);
                            stdout_output.push('\n');
                        }

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
