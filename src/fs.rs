use std::{
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

pub fn load_lua(path: &str) -> Result<Vec<String>, Error> {
    let root = Path::new(path);
    if !root.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("path `{path}` does not exist"),
        ));
    }

    let mut lua_files = Vec::new();
    collect_lua_files(root, &mut lua_files)?;
    lua_files.sort();

    let mut out = Vec::with_capacity(lua_files.len());
    for file_path in lua_files {
        out.push(fs::read_to_string(file_path)?);
    }

    Ok(out)
}

fn collect_lua_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), Error> {
    if path.is_file() {
        if is_lua_file(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_lua_files(&entry_path, out)?;
        } else if is_lua_file(&entry_path) {
            out.push(entry_path);
        }
    }

    Ok(())
}

fn is_lua_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("lua")
}
