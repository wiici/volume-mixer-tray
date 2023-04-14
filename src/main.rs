mod message_only_window;
mod volume_mixer_process;
mod volume_mixer_tray_icon;
mod windows_utils;

use crate::volume_mixer_tray_icon::VolumeMixerTrayIcon;
use crate::volume_mixer_tray_icon::PROP_VOLUME_MIXER_HWND;
use env_logger::Builder;
use log::info;
use message_only_window::MessageOnlyWindow;
use std::io::Write;
use volume_mixer_process::VolumeMixerProcess;
use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, RemovePropW, SetPropW, TranslateMessage, MSG, WNDPROC,
};

fn main() -> Result<(), String> {
    init_logger();

    let volume_mixer_process = VolumeMixerProcess::new()?;
    info!("Run Volue Mixer with pid {}", volume_mixer_process.pid);

    let msg_only_window = MessageOnlyWindow::new(
        "VolumeMixerWindowClass",
        &WNDPROC::Some(VolumeMixerTrayIcon::wnd_proc),
    )?;
    info!("Create hidden message-only window");

    let utf16_prop_name = PCWSTR::from_raw(
        PROP_VOLUME_MIXER_HWND
            .encode_utf16()
            .collect::<Vec<u16>>()
            .as_ptr(),
    );
    let set_prop_result = unsafe {
        SetPropW(
            msg_only_window.hwnd,
            utf16_prop_name,
            HANDLE {
                0: volume_mixer_process.hwnd.0,
            },
        )
    };
    if let Err(err) = set_prop_result.ok() {
        return Err(format!(
            "RemovePropA failed for {}: {}",
            unsafe { utf16_prop_name.display() },
            err
        ));
    }

    let _volume_mixer_tray_icon = VolumeMixerTrayIcon::new(msg_only_window.hwnd);

    let mut msg = MSG::default();
    loop {
        let result = { unsafe { GetMessageW(&mut msg, msg_only_window.hwnd, 0, 0) } };
        if result.as_bool() {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        } else {
            break;
        }
    }

    let result_handle = unsafe { RemovePropW(msg_only_window.hwnd, utf16_prop_name) };

    if result_handle.is_err() {
        return Err(format!(
            "RemovePropA failed for {}: {}",
            unsafe { utf16_prop_name.display() },
            Error::from_win32()
        ));
    }

    Ok(())
}

fn init_logger() {
    let _ = Builder::from_default_env()
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());

            writeln!(
                buf,
                "[{} {} {} {}] {}",
                buf.timestamp(),
                std::process::id(),
                std::thread::current().name().unwrap_or("<unknwon thread>"),
                style.value(record.level()),
                record.args()
            )
        })
        .format_timestamp_millis()
        .init();
}
