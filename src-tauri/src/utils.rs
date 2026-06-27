use std::process::Command;

#[cfg(target_os = "windows")]
pub fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

#[cfg(not(target_os = "windows"))]
pub fn hide_window(_cmd: &mut Command) {}

pub fn is_command_available(cmd: &str) -> bool {
    let check_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let mut command = Command::new(check_cmd);
    command.arg(cmd);
    hide_window(&mut command);
    command
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn query_registry(full_key: &str, value_name: &str) -> Option<String> {
    let parts: Vec<&str> = full_key.splitn(2, '\\').collect();
    if parts.len() != 2 {
        return None;
    }
    let hkey_root = parts[0];
    let sub_key = parts[1];

    use std::os::raw::c_void;

    #[allow(clippy::upper_case_acronyms)]
    type HKEY = *mut c_void;
    #[allow(clippy::upper_case_acronyms)]
    type LSTATUS = i32;

    const HKEY_LOCAL_MACHINE: HKEY = 0x80000002 as HKEY;
    const HKEY_CURRENT_USER: HKEY = 0x80000001 as HKEY;
    const KEY_READ: u32 = 0x20019;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hKey: HKEY,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut HKEY,
        ) -> LSTATUS;

        fn RegQueryValueExW(
            hKey: HKEY,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> LSTATUS;

        fn RegCloseKey(hKey: HKEY) -> LSTATUS;
    }

    let root = match hkey_root {
        "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        _ => return None,
    };

    let sub_key_w: Vec<u16> = sub_key.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_w: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut hkey: HKEY = std::ptr::null_mut();
    unsafe {
        if RegOpenKeyExW(root, sub_key_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }

        let mut value_type: u32 = 0;
        let mut data_len: u32 = 0;

        if RegQueryValueExW(
            hkey,
            value_name_w.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut data_len,
        ) != 0
        {
            RegCloseKey(hkey);
            return None;
        }

        if value_type != REG_SZ && value_type != REG_EXPAND_SZ {
            RegCloseKey(hkey);
            return None;
        }

        let mut buf = vec![0u8; data_len as usize];
        if RegQueryValueExW(
            hkey,
            value_name_w.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            buf.as_mut_ptr(),
            &mut data_len,
        ) != 0
        {
            RegCloseKey(hkey);
            return None;
        }

        RegCloseKey(hkey);

        let u16_len = (data_len as usize) / 2;
        if u16_len == 0 {
            return None;
        }
        let u16_buf = std::slice::from_raw_parts(buf.as_ptr() as *const u16, u16_len);
        let end = u16_buf.iter().position(|&x| x == 0).unwrap_or(u16_len);
        if let Ok(s) = String::from_utf16(&u16_buf[..end]) {
            let mut cleaned_val = s;
            if cleaned_val.starts_with('"') && cleaned_val.ends_with('"') && cleaned_val.len() >= 2
            {
                cleaned_val.remove(0);
                cleaned_val.pop();
            }
            return Some(cleaned_val.trim().to_string());
        }
    }
    None
}

pub fn clean_path(path: &str) -> String {
    let mut s = path.trim().to_string();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s.remove(0);
        s.pop();
    }
    s.trim().to_string()
}

/// 如果目录下有且仅有一个子目录且无其他内容，将该子目录内容提升至顶层，并删除空子目录。
/// 该过程会循环进行直到不满足条件。
pub fn flatten_single_subdir(dir_path: &str) -> Result<(), String> {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    let dir = Path::new(dir_path);
    if !dir.is_dir() {
        return Err(format!("路径不是一个目录: {}", dir_path));
    }

    loop {
        let mut entries = Vec::new();
        let read_dir_result =
            fs::read_dir(dir).map_err(|e| format!("无法读取目录 {}: {}", dir_path, e))?;
        for entry in read_dir_result {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with('.')
                || name_str.eq_ignore_ascii_case("thumbs.db")
                || name_str.eq_ignore_ascii_case("desktop.ini")
            {
                continue;
            }
            entries.push(entry);
        }

        if entries.len() != 1 {
            break;
        }

        let single_entry = &entries[0];
        let sub_dir_path = single_entry.path();
        if !sub_dir_path.is_dir() {
            break;
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_dir_name = format!("temp_flatten_{}", nanos);
        let temp_dir_path = dir.join(&temp_dir_name);

        fs::rename(&sub_dir_path, &temp_dir_path)
            .map_err(|e| format!("重命名子目录 {:?} 到临时目录失败: {}", sub_dir_path, e))?;

        let read_temp_result =
            fs::read_dir(&temp_dir_path).map_err(|e| format!("读取临时目录失败: {}", e))?;

        for entry in read_temp_result {
            let entry = entry.map_err(|e| format!("读取临时目录项失败: {}", e))?;
            let entry_path = entry.path();
            if let Some(file_name) = entry_path.file_name() {
                let dest_path = dir.join(file_name);
                fs::rename(&entry_path, &dest_path).map_err(|e| {
                    format!("移动文件 {:?} 到 {:?} 失败: {}", entry_path, dest_path, e)
                })?;
            }
        }

        fs::remove_dir(&temp_dir_path)
            .map_err(|e| format!("删除临时空目录 {:?} 失败: {}", temp_dir_path, e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_flatten_single_subdir() {
        let temp_dir = std::env::temp_dir().join("test_flatten_unzip");
        if temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        fs::create_dir_all(&temp_dir).unwrap();

        // 构造嵌套结构:
        // temp_dir
        // - sub_1
        //   - .DS_Store (隐藏垃圾文件，应该被忽略以允许 sub_2 提升)
        //   - sub_2
        //     - file1.txt
        //     - file2.txt
        // - desktop.ini (顶层隐藏垃圾文件，应该被忽略以允许 sub_1 提升)
        let sub_1 = temp_dir.join("sub_1");
        let sub_2 = sub_1.join("sub_2");
        fs::create_dir_all(&sub_2).unwrap();

        fs::write(sub_2.join("file1.txt"), "hello").unwrap();
        fs::write(sub_2.join("file2.txt"), "world").unwrap();
        fs::write(sub_1.join(".DS_Store"), "garbage").unwrap();
        fs::write(temp_dir.join("desktop.ini"), "sysinfo").unwrap();

        // 运行提升
        let res = flatten_single_subdir(temp_dir.to_str().unwrap());
        assert!(res.is_ok());

        // 提升后应该直接是:
        // temp_dir
        // - file1.txt
        // - file2.txt
        // - .DS_Store (从 sub_1 转移上来)
        // - desktop.ini
        // sub_1 和 sub_2 应该都被删除
        assert!(temp_dir.join("file1.txt").exists());
        assert!(temp_dir.join("file2.txt").exists());
        assert!(temp_dir.join(".DS_Store").exists());
        assert!(temp_dir.join("desktop.ini").exists());
        assert!(!sub_1.exists());
        assert!(!sub_2.exists());

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
