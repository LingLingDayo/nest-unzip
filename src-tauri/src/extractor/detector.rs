use crate::utils::{clean_path, is_command_available};
use std::path::Path;

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
            if is_command_available("bz") {
                return Some("bz".to_string());
            }
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
                        let bz_path = base_path.join("bz.exe");
                        if bz_path.exists() {
                            return Some(bz_path.to_string_lossy().to_string());
                        }
                        let bc_path = base_path.join("bc.exe");
                        if bc_path.exists() {
                            return Some(bc_path.to_string_lossy().to_string());
                        }
                        let gui_path = base_path.join("Bandizip.exe");
                        if gui_path.exists() {
                            return Some(gui_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
            let paths = [
                "C:\\Program Files\\Bandizip\\bz.exe",
                "C:\\Program Files\\Bandizip\\bc.exe",
                "C:\\Program Files\\Bandizip\\Bandizip.exe",
                "C:\\Program Files (x86)\\Bandizip\\bz.exe",
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

    // 1. 如果本身是一个存在的文件，且不是 Bandizip.exe GUI 则直接返回
    if path.is_file() && path.exists() {
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            let lower = file_name.to_lowercase();
            if lower == "bandizip.exe" {
                if let Some(parent) = path.parent() {
                    let bz_path = parent.join("bz.exe");
                    if bz_path.exists() {
                        return Some(bz_path.to_string_lossy().to_string());
                    }
                    let bc_path = parent.join("bc.exe");
                    if bc_path.exists() {
                        return Some(bc_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        return Some(cleaned);
    }

    // 2. 如果是目录，在里面寻找对应的 exe
    let exe_names = if exe_type == "7z" {
        vec!["7z.exe", "7Z.exe"]
    } else {
        vec![
            "bz.exe",
            "bc.exe",
            "Bandizip.exe",
            "BZ.exe",
            "BC.exe",
            "BANDIZIP.exe",
        ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_exe_path_bandizip_redirect() {
        let gui_path = "H:\\APP\\Bandizip\\Bandizip.exe";
        if std::path::Path::new(gui_path).exists() {
            let resolved = resolve_exe_path(gui_path, "bandizip").unwrap();
            assert!(resolved.to_lowercase().ends_with("bz.exe") || resolved.to_lowercase().ends_with("bc.exe"));
            println!("成功重定向 Bandizip.exe 到: {}", resolved);
        }
    }
}
