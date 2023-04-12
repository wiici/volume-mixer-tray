mod message_only_window;
mod notif_icon;
mod volume_mixer_process;
mod windows_utils;

use env_logger::Builder;
use log::info;
use message_only_window::MessageOnlyWindow;
use std::io::Write;
use volume_mixer_process::VolumeMixerProcess;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WNDPROC};

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, umsg, wparam, lparam)
}

fn main() -> Result<(), String> {
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

    let volume_mixer_process = VolumeMixerProcess::new()?;
    info!("Run Volue Mixer with pid {}", volume_mixer_process.pid);

    let _msg_only_window =
        MessageOnlyWindow::new("VolumeMixerWindowClass", &WNDPROC::Some(wnd_proc))?;
    info!("Create hidden message-only window");

    Ok(())
}
