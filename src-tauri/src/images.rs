use crate::error::{Error, Result};
use base64::Engine;
use image::{codecs::png::PngEncoder, ImageEncoder};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
pub fn save(dir: &Path, data: arboard::ImageData<'_>, limit: usize) -> Result<(String, String)> {
    let expected = data
        .width
        .checked_mul(data.height)
        .and_then(|v| v.checked_mul(4))
        .ok_or_else(|| Error::Invalid("Image dimensions overflow".into()))?;
    if expected > limit || expected != data.bytes.len() {
        return Err(Error::Invalid("Image exceeds size limit".into()));
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(data.width as u64).to_le_bytes());
    hasher.update(&(data.height as u64).to_le_bytes());
    hasher.update(&data.bytes);
    let hash = hasher.finalize().to_hex().to_string();
    let path = dir.join(format!("{hash}.png"));
    if !path.exists() {
        let tmp = dir.join(format!("{hash}.tmp"));
        let result: Result<()> = (|| {
            let file = std::fs::File::create(&tmp)?;
            PngEncoder::new(file)
                .write_image(
                    &data.bytes,
                    data.width as u32,
                    data.height as u32,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|_| Error::Invalid("Cannot encode image".into()))?;
            std::fs::rename(&tmp, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result?;
    }
    Ok((
        path.file_name()
            .ok_or_else(|| Error::Invalid("Invalid image path".into()))?
            .to_string_lossy()
            .into_owned(),
        hash,
    ))
}
fn path(dir: &Path, name: &str) -> Result<PathBuf> {
    if name.len() != 68
        || !name.ends_with(".png")
        || !name[..64].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::Invalid("Invalid stored image name".into()));
    }
    Ok(dir.join(name))
}
pub fn load(dir: &Path, name: &str) -> Result<arboard::ImageData<'static>> {
    let path = path(dir, name)?;
    if std::fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
        return Err(Error::Invalid("Stored image exceeds size limit".into()));
    }
    decode(std::fs::read(path)?)
}
pub fn thumbnail(dir: &Path, name: &str) -> Result<String> {
    let data = load(dir, name)?;
    let rgba = image::RgbaImage::from_raw(
        data.width as u32,
        data.height as u32,
        data.bytes.into_owned(),
    )
    .ok_or_else(|| Error::Invalid("Invalid pixels".into()))?;
    let thumb = image::DynamicImage::ImageRgba8(rgba)
        .thumbnail(480, 320)
        .to_rgba8();
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            thumb.as_raw(),
            thumb.width(),
            thumb.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| Error::Invalid("Cannot create preview".into()))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
pub fn reconcile(dir: &Path, referenced: Vec<String>) -> Result<()> {
    let keep: std::collections::HashSet<_> = referenced.into_iter().collect();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if (name.ends_with(".png") || name.ends_with(".tmp")) && !keep.contains(&name) {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn decode(bytes: Vec<u8>) -> Result<arboard::ImageData<'static>> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| Error::Invalid("Invalid image format".into()))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    reader.limits(limits);
    let rgba = reader
        .decode()
        .map_err(|_| Error::Invalid("Image decoding failed or size limit exceeded".into()))?
        .to_rgba8();
    Ok(arboard::ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_thumbnail_cleanup_and_path_validation() {
        let dir = std::env::temp_dir().join(format!("cv-image-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pixels = vec![255u8; 16];
        let (name, _) = save(
            &dir,
            arboard::ImageData {
                width: 2,
                height: 2,
                bytes: Cow::Borrowed(&pixels),
            },
            1024,
        )
        .unwrap();
        let loaded = load(&dir, &name).unwrap();
        assert_eq!(loaded.bytes.as_ref(), pixels.as_slice());
        assert!(thumbnail(&dir, &name)
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(load(&dir, "../../secret").is_err());
        assert!(save(
            &dir,
            arboard::ImageData {
                width: 100,
                height: 100,
                bytes: Cow::Borrowed(&pixels)
            },
            1024
        )
        .is_err());
        reconcile(&dir, vec![name.clone()]).unwrap();
        assert!(dir.join(name).exists());
        reconcile(&dir, vec![]).unwrap();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        std::fs::remove_dir(dir).unwrap();
    }
}
