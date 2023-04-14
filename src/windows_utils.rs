#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::{Error, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, SetLastError, BOOL, ERROR_NO_MORE_FILES, ERROR_SUCCESS, FALSE, HANDLE, HWND,
    INVALID_HANDLE_VALUE, LPARAM, TRUE, WIN32_ERROR,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    CreateProcessW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextA, GetWindowThreadProcessId, WNDENUMPROC,
};

pub struct WindowsHandle {
    handle: HANDLE,
}

impl WindowsHandle {
    const NULL: HANDLE = HANDLE(0);

    pub fn is_valid(&self) -> bool {
        self.handle == INVALID_HANDLE_VALUE || self.handle == WindowsHandle::NULL
    }

    pub fn from_raw_handle(handle: HANDLE) -> Self {
        WindowsHandle { handle }
    }

    pub fn as_raw_handle(&self) -> HANDLE {
        self.handle
    }
}

impl Default for WindowsHandle {
    fn default() -> Self {
        WindowsHandle {
            handle: INVALID_HANDLE_VALUE,
        }
    }
}

impl Drop for WindowsHandle {
    fn drop(&mut self) {
        if self.is_valid() {
            unsafe {
                CloseHandle(self.handle).ok().unwrap();
            }
        }
    }
}

pub trait ExtendPCWSTR {
    fn from_str(str: &str) -> PCWSTR;
}

impl ExtendPCWSTR for PCWSTR {
    fn from_str(str: &str) -> PCWSTR {
        PCWSTR::from_raw(
            str.encode_utf16()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
        )
    }
}

pub fn get_pid_by_name(proc_name: &str) -> Result<Option<u32>, String> {
    let mut result_pid: Option<u32> = None;

    unsafe {
        let snapshot_handle = {
            let raw_handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
            WindowsHandle::from_raw_handle(raw_handle)
        };

        let mut proc_entry = PROCESSENTRY32W::default();
        proc_entry.dwSize = std::mem::size_of_val(&proc_entry) as u32;

        if Process32FirstW(snapshot_handle.as_raw_handle(), &mut proc_entry).as_bool() {
            loop {
                let curr_proc_name = String::from_utf16(proc_entry.szExeFile.as_slice()).unwrap();
                if proc_name == curr_proc_name.trim_end_matches('\0') {
                    result_pid = Some(proc_entry.th32ProcessID);
                    break;
                }
                if !Process32NextW(snapshot_handle.as_raw_handle(), &mut proc_entry).as_bool() {
                    let error = Error::from_win32();
                    if let Some(win32_error) = WIN32_ERROR::from_error(&error) {
                        if win32_error == ERROR_NO_MORE_FILES {
                            break;
                        } else {
                            return Err(format!("Process32NextW failed: {}", error));
                        }
                    }
                }
            }
        } else {
            return Err(format!("Process32FirstW failed: {}", Error::from_win32()));
        }
    }

    Ok(result_pid)
}

#[derive(Default)]
struct EnumProcUserData {
    looking_pid: u32,
    title_pattern: String,
    found_hwnd: Option<HWND>,
}

unsafe extern "system" fn enum_windows_proc(curr_hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut user_data = (lparam.0 as *mut EnumProcUserData).as_mut().unwrap();
    let mut curr_hwnd_pid: u32 = 0;

    unsafe {
        let result = GetWindowThreadProcessId(curr_hwnd, Some(&mut curr_hwnd_pid));
        if result == 0 {
            warn!(
                "Failed to call GetWindowThreadProcessId. Windows error: {}",
                Error::from_win32()
            );

            return TRUE;
        }
    }

    let mut window_text_buf: [u8; 256] = [0; 256];
    let ret_buf_len = GetWindowTextA(curr_hwnd, window_text_buf.as_mut());
    if ret_buf_len == 0 {
        warn!("Failed to get window text: {}", Error::from_win32());
    }
    let curr_window_title = std::str::from_utf8(window_text_buf.as_slice()).unwrap_or("<unknown>");

    if curr_hwnd_pid == user_data.looking_pid
        && curr_window_title.contains(user_data.title_pattern.as_str())
    {
        user_data.found_hwnd = Some(curr_hwnd);
        SetLastError(ERROR_SUCCESS);
        FALSE
    } else {
        TRUE
    }
}

pub fn find_window_by_pattern_in_title(
    pattern: &str,
    looking_window_owner_pid: u32,
) -> Result<HWND, String> {
    let mut user_data = EnumProcUserData {
        looking_pid: looking_window_owner_pid,
        found_hwnd: None,
        title_pattern: pattern.to_string(),
    };
    let lparam = LPARAM(&mut user_data as *mut EnumProcUserData as isize);

    let enum_result = unsafe { EnumWindows(WNDENUMPROC::Some(enum_windows_proc), lparam) };

    if let Err(err) = enum_result.ok() {
        if err.code() == ERROR_SUCCESS.to_hresult() {
            Ok(user_data.found_hwnd.expect("In this case window handle HWND should be set. Check callback function passed to EnumWindows"))
        } else {
            Err(format!("EnumWindows failed: {}", err))
        }
    } else {
        Err(format!(
            "Could not find window title with pattern \"{}\" owned by pid {}",
            pattern, looking_window_owner_pid
        ))
    }
}

pub fn run_exec(
    exec_path: &Path,
    startup_info: &STARTUPINFOW,
) -> Result<(u32, WindowsHandle), String> {
    let mut process_info = PROCESS_INFORMATION::default();
    let mut utf16_exec_path: Vec<u16> =
        exec_path.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        let process_result = CreateProcessW(
            PCWSTR::null(),
            PWSTR(utf16_exec_path.as_mut_ptr()),
            None,
            None,
            FALSE,
            PROCESS_CREATION_FLAGS::default(),
            None,
            PCWSTR::null(),
            startup_info,
            &mut process_info,
        );

        if process_result.as_bool() {
            debug!(
                "Run exec (pid {}) \"{}\"",
                process_info.dwProcessId,
                exec_path.display()
            );

            CloseHandle(process_info.hThread).ok().unwrap();

            Ok((
                process_info.dwProcessId,
                WindowsHandle::from_raw_handle(process_info.hProcess),
            ))
        } else {
            Err(format!("CreateProcessW failed: {}", Error::from_win32()))
        }
    }
}
