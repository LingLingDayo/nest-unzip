use super::archive::{extract_single_archive, find_archives_in_dir};
use super::detector::resolve_exe_path;
use crate::types::LogPayload;
use tauri::Emitter;

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
        } else if let Ok(nested_archives) = find_archives_in_dir(&target_dir) {
            for archive in nested_archives {
                let _ = trash::delete(std::path::Path::new(&archive));
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

        if nested_archives.len() >= 2 {
            let names: Vec<String> = nested_archives
                .iter()
                .map(|p| {
                    std::path::Path::new(p)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("未知压缩包")
                        .to_string()
                })
                .collect();
            emit_log(
                &format!(
                    "第 {} 层扫描找到 {} 个嵌套压缩包: {}，为避免混乱，跳过对这些嵌套包的解压。",
                    depth,
                    nested_archives.len(),
                    names.join(", ")
                ),
                "success",
                100.0,
            );
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
