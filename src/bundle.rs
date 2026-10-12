use super::*;
use std::io::{Cursor, Read, Write};
pub(super) fn read(path: &Path) -> Result<(SavedDocument, Vec<(String, Vec<u8>)>), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"PK") {
        return serde_json::from_slice(&bytes)
            .map(|document| (document, Vec::new()))
            .map_err(|e| e.to_string());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() > 1000 {
        return Err("This document bundle has too many files.".into());
    }
    let mut document = Vec::new();
    archive
        .by_name("document.json")
        .map_err(|e| e.to_string())?
        .take(10 * 1024 * 1024)
        .read_to_end(&mut document)
        .map_err(|e| e.to_string())?;
    let document: SavedDocument = serde_json::from_slice(&document).map_err(|e| e.to_string())?;
    if document.version > 2 {
        return Err("This document uses a newer flowrite format.".into());
    }
    let mut assets = Vec::new();
    let mut total = 0;
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        if let Some(name) = file.name().strip_prefix("assets/").map(str::to_owned) {
            if !images::safe_asset(&name) || file.size() > 30 * 1024 * 1024 {
                return Err("Unsafe image entry in document bundle.".into());
            }
            let mut bytes = Vec::new();
            file.take(30 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            total += bytes.len();
            if total > 200 * 1024 * 1024 {
                return Err("Document images exceed the import size limit.".into());
            }
            assets.push((name, bytes));
        }
    }
    Ok((document, assets))
}
impl FlowriteApp {
    pub(super) fn write_bundle(&self, path: &Path) -> Result<(), String> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut document = self.local_document();
        for block in &mut document.blocks {
            wikilinks::store(block, &self.workspace);
        }
        zip.start_file("document.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut names = std::collections::HashSet::new();
        for block in &self.blocks {
            if let Some(asset) = &block.image {
                if asset.name.is_empty() || !names.insert(&asset.name) {
                    continue;
                }
                if !images::safe_asset(&asset.name) {
                    return Err("Unsafe image asset name".into());
                }
                let bytes = std::fs::read(
                    images::asset_directory(&self.workspace, &self.active_document)
                        .join(&asset.name),
                )
                .map_err(|e| e.to_string())?;
                zip.start_file(format!("assets/{}", asset.name), options)
                    .map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
        let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
        workspace::write_atomic(path, &bytes).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundles_round_trip_embedded_assets() {
        let mut app = FlowriteApp::empty();
        app.workspace.directory =
            std::env::temp_dir().join(format!("flowrite-bundle-{}", workspace::timestamp()));
        app.dirty = true;
        app.commit_current();
        let mut block = Block::new(2, BlockKind::Image, "");
        block.image = Some(images::ImageAsset {
            name: "image.png".into(),
            ..Default::default()
        });
        app.blocks.push(block);
        let folder = images::asset_directory(&app.workspace, &app.active_document);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("image.png"), b"test-image-bytes").unwrap();
        let path = app.workspace.directory.join("copy.flowrite");
        app.write_bundle(&path).unwrap();
        let (document, assets) = read(&path).unwrap();
        assert_eq!(document.blocks.len(), 2);
        assert_eq!(
            assets,
            vec![("image.png".into(), b"test-image-bytes".to_vec())]
        );
    }
}
