use std::ffi::c_void;
use crate::windows_utils::ExtendPCWSTR;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::{FALSE, HANDLE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconA, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetPropW, GetWindowRect, IsWindowVisible, LoadIconW, MoveWindow,
    PostQuitMessage, SetForegroundWindow, ShowWindow, SystemParametersInfoA, IDI_APPLICATION,
    SPI_GETWORKAREA, SW_HIDE, SW_SHOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WM_APP, WM_LBUTTONDOWN,
    WM_RBUTTONDOWN,
};

pub const PROP_VOLUME_MIXER_HWND: &str = "PROP_VOLUME_MIXER_HWND";

#[derive(Default)]
pub struct VolumeMixerTrayIcon {
    notif_data: NOTIFYICONDATAA,
}

impl VolumeMixerTrayIcon {
    pub const MSG_ID: u32 = WM_APP + 1;

    pub fn new(hwnd: HWND) -> VolumeMixerTrayIcon {
        let tip_msg_buf = Self::construct_tip_msg_buf("Custom Volume Mixer");

        let mut notif_data = NOTIFYICONDATAA::default();

        notif_data.cbSize = std::mem::size_of_val(&notif_data) as u32;
        notif_data.hWnd = hwnd;
        notif_data.uFlags = NIF_TIP | NIF_ICON | NIF_MESSAGE;
        notif_data.uCallbackMessage = Self::MSG_ID;
        notif_data.szTip = tip_msg_buf;
        notif_data.hIcon = unsafe { LoadIconW(HMODULE::default(), IDI_APPLICATION).unwrap() };
        let notif_result = unsafe { Shell_NotifyIconA(NIM_ADD, &notif_data) };
        if notif_result.as_bool() {
            info!("Send message to add icon");
        } else {
            error!("Failed to send message that adds icon");
        }

        VolumeMixerTrayIcon { notif_data }
    }

    pub unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        umsg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match umsg {
            VolumeMixerTrayIcon::MSG_ID => match lparam.0 as u32 {
                WM_LBUTTONDOWN => Self::on_left_mouse_pressed(hwnd),
                WM_RBUTTONDOWN => Self::on_right_mouse_pressed(),
                _ => {}
            },
            _ => return DefWindowProcW(hwnd, umsg, wparam, lparam),
        }

        LRESULT::default()
    }

    fn construct_tip_msg_buf(tip_msg: &str) -> [u8; 128] {
        let mut array: [u8; 128] = [0; 128];
        array[..tip_msg.len()].copy_from_slice(tip_msg.as_bytes());

        array
    }

    fn on_left_mouse_pressed(hwnd: HWND) {
        let utf16_prop_name = PCWSTR::from_str(PROP_VOLUME_MIXER_HWND);
        let data: HANDLE = { unsafe { GetPropW(hwnd, utf16_prop_name) } };

        if data.is_invalid() {
            error!(
                "GetPropW failed for property \"{}\": {}",
                unsafe { utf16_prop_name.display() },
                Error::from_win32()
            );

            return;
        }

        let volume_mixer_hwnd = HWND(data.0);
        let is_visible = unsafe { IsWindowVisible(volume_mixer_hwnd).as_bool() };
        let show_cmd = {
            if is_visible {
                SW_HIDE
            } else {
                if let Err(err_str) = Self::move_window_to_right_bottom_corner(volume_mixer_hwnd) {
                    error!("Failed to move volume mixer window. Reason: {}", err_str);
                }

                SW_SHOW
            }
        };
        unsafe {
            ShowWindow(volume_mixer_hwnd, show_cmd);
            SetForegroundWindow(volume_mixer_hwnd);
        }
    }

    fn on_right_mouse_pressed() {
        unsafe { PostQuitMessage(0) };
    }

    fn move_window_to_right_bottom_corner(hwnd: HWND) -> Result<(), String> {
        let (window_width, window_height) = Self::get_window_size(hwnd)?;
        let (desktop_width, desktop_height) = Self::get_desktop_size_without_taskbar()?;

        let move_result = unsafe {
            MoveWindow(
                hwnd,
                desktop_width - window_width,
                desktop_height - window_height,
                window_width,
                window_height,
                FALSE,
            )
        };
        if let Err(err) = move_result.ok() {
            Err(format!("MoveWindow failed: {}", err))
        } else {
            Ok(())
        }
    }

    fn get_window_size(hwnd: HWND) -> Result<(i32, i32), String> {
        let mut window_rect = RECT::default();
        let get_rect_result = unsafe { GetWindowRect(hwnd, &mut window_rect) };
        if let Err(err) = get_rect_result.ok() {
            Err(format!("GetWindowRect failed: {}", err))
        } else {
            Ok((
                window_rect.right - window_rect.left,
                window_rect.bottom - window_rect.top,
            ))
        }
    }

    fn get_desktop_size_without_taskbar() -> Result<(i32, i32), String> {
        let mut desktop_rect = RECT::default();
        let sysinfo_req_result = unsafe {
            SystemParametersInfoA(
                SPI_GETWORKAREA,
                0,
                Some(&mut desktop_rect as *mut _ as *mut c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
            )
        };
        if let Err(err) = sysinfo_req_result.ok() {
            Err(format!("SystemParametersInfoA failed: {}", err))
        } else {
            Ok((
                desktop_rect.right - desktop_rect.left,
                desktop_rect.bottom - desktop_rect.top,
            ))
        }
    }
}

impl Drop for VolumeMixerTrayIcon {
    fn drop(&mut self) {
        let notif_delete_result = unsafe { Shell_NotifyIconA(NIM_DELETE, &self.notif_data) };
        if notif_delete_result.as_bool() {
            info!("Send message to delete icon");
        } else {
            warn!("Failed to send message that deletes icon");
        }
    }
}
