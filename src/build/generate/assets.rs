use std::{fs, path::Path};

use crate::error::MangoError;

pub fn build(assets_source: &Path, asset_dest: &Path) -> Result<(), MangoError> {
    if !assets_source.is_dir() {
        let msg = format!("the path '{}' is not a directory", assets_source.to_str().unwrap_or_default());
        return Err(MangoError::General(msg));
    }

    fs::create_dir_all(asset_dest)?;

    let children = fs::read_dir(assets_source)?;
    for child in children {
        let child = child?;
        let filename = &child.file_name();
        let name = Path::new(filename);
        let source_path = child.path();
        let dest_path = asset_dest.join(name);

        if child.file_type()?.is_dir() {
            
            return build(&source_path, &dest_path);
        } 

        match fs::copy(&source_path, &dest_path) {
            Err(e) => {
                println!("{}", e.to_string());
            },
            _ => ()
        }
        

        

    }

    Ok(())
}