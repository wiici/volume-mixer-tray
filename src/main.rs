mod notif_icon;
mod volume_mixer_process;
mod windows_utils;

use volume_mixer_process::VolumeMixerProcess;

fn main() -> Result<(), String> {
    let volume_mixer_process = VolumeMixerProcess::new().unwrap();
    println!(
        "Get window handle for volume mixer process (pid {})",
        volume_mixer_process.pid
    );

    Ok(())
}
