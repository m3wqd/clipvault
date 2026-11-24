use crate::error::{Error, Result};
pub enum Backend {
    Native(arboard::Clipboard),
    #[cfg(target_os = "linux")]
    Wayland,
}
impl Backend {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return Ok(Self::Wayland);
        }
        arboard::Clipboard::new()
            .map(Self::Native)
            .map_err(|_| Error::Invalid("Native clipboard unavailable".into()))
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Native(_) => {
                if cfg!(windows) {
                    "Windows / arboard"
                } else {
                    "X11 / arboard"
                }
            }
            #[cfg(target_os = "linux")]
            Self::Wayland => "Wayland / wl-clipboard",
        }
    }
    pub fn read(&mut self) -> Result<Option<String>> {
        match self {
            Self::Native(c) => match c.get_text() {
                Ok(s) => Ok(Some(s)),
                Err(arboard::Error::ContentNotAvailable) => Ok(None),
                Err(_) => Err(Error::Invalid("Clipboard temporarily unavailable".into())),
            },
            #[cfg(target_os = "linux")]
            Self::Wayland => {
                let types = wayland::read(&["--list-types"], 4096)?;
                if !String::from_utf8_lossy(&types)
                    .lines()
                    .any(|s| s.starts_with("text/") || s == "UTF8_STRING")
                {
                    return Ok(None);
                }
                let bytes = wayland::read(&["--no-newline", "--type", "text"], 16 * 1024 * 1024)?;
                String::from_utf8(bytes)
                    .map(Some)
                    .map_err(|_| Error::Invalid("Clipboard text is not UTF-8".into()))
            }
        }
    }
    pub fn write(&mut self, text: String) -> Result<()> {
        match self {
            Self::Native(c) => c
                .set_text(text)
                .map_err(|_| Error::Invalid("Cannot write clipboard".into())),
            #[cfg(target_os = "linux")]
            Self::Wayland => wayland::write("text/plain;charset=utf-8", text.into_bytes()),
        }
    }
    pub fn read_image(&mut self) -> Result<Option<arboard::ImageData<'static>>> {
        match self {
            Self::Native(c) => match c.get_image() {
                Ok(i) => Ok(Some(i)),
                Err(arboard::Error::ContentNotAvailable) => Ok(None),
                Err(_) => Err(Error::Invalid("Image clipboard unavailable".into())),
            },
            #[cfg(target_os = "linux")]
            Self::Wayland => {
                let types = wayland::read(&["--list-types"], 4096)?;
                if !String::from_utf8_lossy(&types)
                    .lines()
                    .any(|t| t == "image/png")
                {
                    return Ok(None);
                }
                let bytes = wayland::read(&["--type", "image/png"], 16 * 1024 * 1024)?;
                crate::images::decode(bytes).map(Some)
            }
        }
    }
    pub fn write_image(&mut self, data: arboard::ImageData<'static>) -> Result<()> {
        match self {
            Self::Native(c) => c
                .set_image(data)
                .map_err(|_| Error::Invalid("Cannot restore image".into())),
            #[cfg(target_os = "linux")]
            Self::Wayland => {
                use image::ImageEncoder;
                let mut bytes = Vec::new();
                image::codecs::png::PngEncoder::new(&mut bytes)
                    .write_image(
                        &data.bytes,
                        data.width as u32,
                        data.height as u32,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|_| Error::Invalid("Cannot encode clipboard image".into()))?;
                wayland::write("image/png", bytes)
            }
        }
    }
}
#[cfg(target_os = "linux")]
mod wayland {
    use super::*;
    use std::{
        io::{Read, Write},
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };
    fn wait(child: &mut Child) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(status) = child.try_wait()? {
                return if status.success() {
                    Ok(())
                } else {
                    Err(Error::Invalid("Wayland clipboard unavailable or empty. Check wl-clipboard and compositor permissions.".into()))
                };
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Invalid("Wayland clipboard command timed out".into()));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    pub fn read(args: &[&str], limit: usize) -> Result<Vec<u8>> {
        let mut child = Command::new("wl-paste")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid("Clipboard pipe unavailable".into()))?;
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take((limit + 1) as u64)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let status = wait(&mut child);
        let bytes = reader
            .join()
            .map_err(|_| Error::Invalid("Clipboard reader failed".into()))??;
        status?;
        if bytes.len() > limit {
            return Err(Error::Invalid("Clipboard exceeds byte limit".into()));
        }
        Ok(bytes)
    }
    pub fn write(mime: &str, bytes: Vec<u8>) -> Result<()> {
        let mut child = Command::new("wl-copy")
            .args(["--type", mime])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid("Clipboard pipe unavailable".into()))?;
        let writer = std::thread::spawn(move || stdin.write_all(&bytes));
        let result = wait(&mut child);
        writer
            .join()
            .map_err(|_| Error::Invalid("Clipboard writer failed".into()))??;
        result
    }
}

/// Best-effort clipboard owner attribution; unavailable owners remain unknown.
#[cfg(windows)]
pub fn source_app() -> Option<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            DataExchange::GetClipboardOwner,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::WindowsAndMessaging::GetWindowThreadProcessId,
    };
    // SAFETY: calls use an OS-provided HWND, checked process handle, and bounded UTF-16 buffer.
    unsafe {
        let owner = GetClipboardOwner();
        if owner.is_null() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(owner, &mut pid);
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().map(str::to_owned)
    }
}
#[cfg(not(windows))]
pub fn source_app() -> Option<String> {
    None
}
