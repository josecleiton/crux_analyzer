use std::fs;
use std::path::Path;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../apps/web/dist"]
pub struct WebAssets;

pub fn export_site(out_dir: &Path) -> Result<(), std::io::Error> {
    if WebAssets::iter().next().is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "embedded web assets are missing",
        ));
    }

    fs::create_dir_all(out_dir)?;

    for file_path in WebAssets::iter() {
        let path_str = file_path.as_ref();
        if path_str == "model.json" {
            continue;
        }

        let dest = out_dir.join(path_str);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = WebAssets::get(path_str).unwrap();
        fs::write(&dest, file.data)?;
    }

    Ok(())
}
