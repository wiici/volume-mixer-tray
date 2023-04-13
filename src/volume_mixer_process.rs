use crate::windows_utils::{
    find_window_by_pattern_in_title, get_pid_by_name, run_exec, WindowsHandle,
};
use core::time;
use log::{info, warn};
use std::env;
use std::path::PathBuf;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::System::Threading::{
    TerminateProcess, STARTF_PREVENTPINNING, STARTF_USESHOWWINDOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

#[derive(Default)]
pub struct VolumeMixerProcess {
    pub pid: u32,
    pub hwnd: HWND,
    hprocess: WindowsHandle,
}

impl VolumeMixerProcess {
    const VOLUME_MIXER_EXEC_NAME: &'static str = "SndVol.exe";

    pub fn new() -> Result<VolumeMixerProcess, String> {
        if Self::is_volume_mixer_running() {
            Self::from_running_process()
        } else {
            Self::from_new_process()
        }
    }

    fn is_volume_mixer_running() -> bool {
        if let Ok(pid_op) = get_pid_by_name(Self::VOLUME_MIXER_EXEC_NAME) {
            pid_op.is_some()
        } else {
            false
        }
    }

    fn from_running_process() -> Result<VolumeMixerProcess, String> {
        if let Some(pid) = get_pid_by_name(Self::VOLUME_MIXER_EXEC_NAME).unwrap() {
            let hwnd = Self::try_find_volume_mixer_window(pid)?;

            Ok(VolumeMixerProcess {
                pid,
                hwnd,
                hprocess: WindowsHandle::default(),
            })
        } else {
            Err(format!(
                "Could not find pid related to exec {}",
                Self::VOLUME_MIXER_EXEC_NAME
            ))
        }
    }

    fn from_new_process() -> Result<VolumeMixerProcess, String> {
        let startup_info = STARTUPINFOW {
            dwFlags: STARTF_PREVENTPINNING | STARTF_USESHOWWINDOW,
            wShowWindow: SW_HIDE.0 as u16,
            ..Default::default()
        };

        let exec_path = Self::construct_volume_mixer_exec_path();

        let (pid, hprocess) = run_exec(exec_path.as_path(), &startup_info)?;

        let hwnd = Self::try_find_volume_mixer_window(pid)?;

        Ok(VolumeMixerProcess {
            pid,
            hwnd,
            hprocess,
        })
    }

    fn try_find_volume_mixer_window(pid: u32) -> Result<HWND, String> {
        // Give OS some time when trying to get HWND
        // just after creating a process.
        let window_title_pattern = "Volume Mixer";
        for _ in 0..4 {
            if let Ok(hwnd) = find_window_by_pattern_in_title(window_title_pattern, pid) {
                return Ok(hwnd);
            } else {
                std::thread::sleep(time::Duration::from_millis(250));
            }
        }

        find_window_by_pattern_in_title(window_title_pattern, pid)
    }

    fn construct_volume_mixer_exec_path() -> PathBuf {
        let env_var_name = "WINDIR";
        let mut exec_path = PathBuf::new();

        let windir = env::var(env_var_name).expect("WINDIR should be available");
        exec_path.push(windir);
        exec_path.push("System32");
        exec_path.push("SndVol.exe");

        assert!(exec_path.exists());

        exec_path
    }
}

impl Drop for VolumeMixerProcess {
    fn drop(&mut self) {
        unsafe {
            if let Err(err) = TerminateProcess(self.hprocess.as_raw_handle(), 0).ok() {
                warn!("TerminateProcess failed: {}", err);
            }
        }
        info!("Terminate Volume Mixer process");
    }
}
