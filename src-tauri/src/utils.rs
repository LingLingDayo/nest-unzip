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
