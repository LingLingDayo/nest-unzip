use super::bandizip::try_extract_bc;
use super::seven_zip::try_extract_7z;

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
