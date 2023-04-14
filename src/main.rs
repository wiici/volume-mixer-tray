mod message_only_window;
mod volume_mixer_process;
mod volume_mixer_tray_icon;
mod windows_utils;

use crate::volume_mixer_tray_icon::VolumeMixerTrayIcon;
use crate::volume_mixer_tray_icon::PROP_VOLUME_MIXER_HWND;
use crate::windows_utils::ExtendPCWSTR;
use env_logger::Builder;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
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

    let set_prop_result = unsafe {
        SetPropW(
            msg_only_window.hwnd,
            PCWSTR::from_str(PROP_VOLUME_MIXER_HWND),
            HANDLE(volume_mixer_process.hwnd.0),
        )
    };
    if let Err(err) = set_prop_result.ok() {
        return Err(format!(
            "SetPropW failed for \"{}\": {}",
            PROP_VOLUME_MIXER_HWND, err
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

    let remove_prop_result = unsafe {
        RemovePropW(
            msg_only_window.hwnd,
            PCWSTR::from_str(PROP_VOLUME_MIXER_HWND),
        )
    };
    if remove_prop_result.is_err() {
        error!(
            "RemovePropW failed for \"{}\": {}",
            PROP_VOLUME_MIXER_HWND,
            Error::from_win32()
        );
    }

    Ok(())
}

fn init_logger() {
    Builder::from_default_env()
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
        .init()
}
