use std::cell::RefCell;
use deno_core::{op2, OpState};
use std::path::PathBuf;
use std::rc::Rc;
use image::ImageFormat;
use image::imageops::FilterType;
use serde::Serialize;
use tokio::task::spawn_blocking;
use crate::plugin_data::PluginData;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum DesktopPathAction {
    #[serde(rename = "add")]
    Add {
        id: String,
        data: DesktopApplication
    },
    #[serde(rename = "remove")]
    Remove {
        id: String
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Serialize)]
pub struct DesktopApplication {
    name: String,
    icon: Option<Vec<u8>>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Serialize)]
pub struct DesktopApplication {
    name: String,
    path: String,
    icon: Option<Vec<u8>>,
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
#[derive(Debug, Serialize)]
pub struct DesktopApplication {

}

#[cfg(target_os = "macos")]
#[derive(Debug, Serialize)]
pub struct DesktopSettingsPre13Data {
    name: String,
    path: String,
    icon: Option<Vec<u8>>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Serialize)]
pub struct DesktopSettings13AndPostData {
    name: String,
    preferences_id: String,
    icon: Option<Vec<u8>>,
}


#[op2]
#[string]
pub fn current_os() -> &'static str {
    std::env::consts::OS
}

#[cfg(target_os = "linux")]
#[op2(async)]
#[serde]
pub async fn linux_app_from_path(state: Rc<RefCell<OpState>>, #[string] path: String) -> anyhow::Result<Option<DesktopPathAction>> {
    let home_dir = {
        let state = state.borrow();

        let home_dir = state
            .borrow::<PluginData>()
            .home_dir();

        home_dir
    };

    Ok(spawn_blocking(|| linux::linux_app_from_path(home_dir, PathBuf::from(path))).await?)
}

#[cfg(target_os = "linux")]
#[op2]
#[serde]
pub fn linux_application_dirs(state: Rc<RefCell<OpState>>) -> Vec<String> {
    let home_dir = {
        let state = state.borrow();

        let home_dir = state
            .borrow::<PluginData>()
            .home_dir();

        home_dir
    };

    linux::linux_application_dirs(home_dir)
        .into_iter()
        .map(|path| path.to_str().expect("non-utf8 paths are not supported").to_string())
        .collect()
}

#[cfg(target_os = "linux")]
#[op2(fast)]
pub fn linux_open_application(#[string] desktop_file_id: String) -> anyhow::Result<()> {

    spawn_detached("gtk-launch", &[desktop_file_id])?;

    Ok(())
}

#[cfg(target_os = "macos")]
#[op2(fast)]
pub fn macos_major_version() -> u8 {
    macos::macos_major_version()
}

#[cfg(target_os = "macos")]
#[op2(async)]
#[serde]
pub async fn macos_app_from_path(#[string] path: String) -> anyhow::Result<Option<DesktopPathAction>> {
    Ok(spawn_blocking(|| macos::macos_app_from_path(&PathBuf::from(path))).await?)
}

#[cfg(target_os = "macos")]
#[op2(async)]
#[serde]
pub async fn macos_app_from_arbitrary_path(#[string] path: String) -> anyhow::Result<Option<DesktopPathAction>> {
    Ok(spawn_blocking(|| macos::macos_app_from_arbitrary_path(PathBuf::from(path))).await?)
}

#[cfg(target_os = "macos")]
#[op2]
#[serde]
pub fn macos_system_applications() -> Vec<String> {
    macos::macos_system_applications()
        .into_iter()
        .map(|path| path.to_str().expect("non-utf8 paths are not supported").to_string())
        .collect()
}

#[cfg(target_os = "macos")]
#[op2]
#[serde]
pub fn macos_application_dirs() -> Vec<String> {
    macos::macos_application_dirs()
        .into_iter()
        .map(|path| path.to_str().expect("non-utf8 paths are not supported").to_string())
        .collect()
}

#[cfg(target_os = "macos")]
#[op2(fast)]
pub fn macos_open_application(#[string] app_path: String) -> anyhow::Result<()> {

    spawn_detached("open", &[app_path])?;

    Ok(())
}

#[cfg(target_os = "macos")]
#[op2]
#[serde]
pub fn macos_settings_pre_13() -> Vec<DesktopSettingsPre13Data> {
    macos::macos_settings_pre_13()
}

#[cfg(target_os = "macos")]
#[op2]
#[serde]
pub fn macos_settings_13_and_post() -> Vec<DesktopSettings13AndPostData> {
    macos::macos_settings_13_and_post()
}

#[cfg(target_os = "macos")]
#[op2(fast)]
pub fn macos_open_setting_13_and_post(#[string] preferences_id: String) -> anyhow::Result<()> {

    spawn_detached(
        "open",
        &[
            format!("x-apple.systempreferences:{}", preferences_id)
        ]
    )?;

    Ok(())
}

#[cfg(target_os = "macos")]
#[op2(fast)]
pub fn macos_open_setting_pre_13(#[string] setting_path: String) -> anyhow::Result<()> {

    spawn_detached(
        "open",
        &[
            "-b",
            "com.apple.systempreferences",
            &setting_path,
        ]
    )?;

    Ok(())
}

#[cfg(unix)]
pub fn spawn_detached<I, S>(
    path: &str,
    args: I,
) -> std::io::Result<()>
where
    I: IntoIterator<Item = S> + Copy,
    S: AsRef<std::ffi::OsStr>,
{
    // from https://github.com/alacritty/alacritty/blob/5abb4b73937b17fe501b9ca20b602950f1218b96/alacritty/src/daemon.rs#L65
    use std::os::unix::prelude::CommandExt;
    use std::process::{Command, Stdio};

    let mut command = Command::new(path);

    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        command
            .pre_exec(|| {
                match libc::fork() {
                    -1 => return Err(std::io::Error::last_os_error()),
                    0 => (),
                    _ => libc::_exit(0),
                }

                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(())
            })
            .spawn()?
            .wait()
            .map(|_| ())
    }
}

pub(in crate::plugins::applications) fn resize_icon(data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let data = image::load_from_memory_with_format(&data, ImageFormat::Png)?;
    let data = image::imageops::resize(&data, 48, 48, FilterType::Lanczos3);

    let mut buffer = std::io::Cursor::new(vec![]);

    data.write_to(&mut buffer, ImageFormat::Png)?;

    Ok(buffer.into_inner())
}