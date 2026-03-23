use std::{
    fs,
    io::{Error, Result, Write},
    path::Path,
};

const DEFS_TEMPLATE: &str = include_str!("../assets/lua/hutc.defs.lua");

pub fn init(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).map_err(|e| {
        Error::new(
            e.kind(),
            format!("failed to create directory `{}`: {e}", path.display()),
        )
    })?;

    let file_path = path.join("hutc.defs.lua");
    let mut file = fs::File::create(&file_path).map_err(|e| {
        Error::new(
            e.kind(),
            format!(
                "failed to create definitions file `{}`: {e}",
                file_path.display()
            ),
        )
    })?;
    file.write_all(DEFS_TEMPLATE.as_bytes()).map_err(|e| {
        Error::new(
            e.kind(),
            format!(
                "failed to write definitions file `{}`: {e}",
                file_path.display()
            ),
        )
    })?;

    Ok(())
}
