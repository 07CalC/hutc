use std::{
    fs,
    io::{Result, Write},
    path::Path,
};

const DEFS_TEMPLATE: &str = include_str!("../assets/lua/hutc.defs.lua");

pub fn init(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path)?;

    let file_path = path.join("hutc.defs.lua");
    let mut file = fs::File::create(file_path)?;
    file.write_all(DEFS_TEMPLATE.as_bytes())?;

    Ok(())
}
