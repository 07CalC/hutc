use std::{
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct LuaSource {
    pub path: PathBuf,
    pub content: String,
}

pub fn load_lua(path: &str) -> Result<Vec<LuaSource>, Error> {
    let root = Path::new(path);
    if !root.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("path `{path}` does not exist"),
        ));
    }

    if root.is_file() && !is_lua_file(root) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "path `{}` is not a Lua file. expected a `.lua` file or a directory containing `.lua` files",
                root.display()
            ),
        ));
    }

    let mut lua_files = Vec::new();
    collect_lua_files(root, &mut lua_files)?;
    lua_files.sort();

    if lua_files.is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("no `.lua` files found under `{}`", root.display()),
        ));
    }

    let mut out = Vec::with_capacity(lua_files.len());
    for file_path in lua_files {
        let content = fs::read_to_string(&file_path).map_err(|e| {
            Error::new(
                e.kind(),
                format!("failed to read Lua file `{}`: {e}", file_path.display()),
            )
        })?;
        out.push(LuaSource {
            path: file_path,
            content,
        });
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

    let entries = fs::read_dir(path).map_err(|e| {
        Error::new(
            e.kind(),
            format!("failed to read directory `{}`: {e}", path.display()),
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "failed to read an entry inside directory `{}`: {e}",
                    path.display()
                ),
            )
        })?;
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
    let is_lua = path.extension().and_then(|s| s.to_str()) == Some("lua");
    if !is_lua {
        return false;
    }
    //skip defination files this time, bcz i am an idiot
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    !file_name.ends_with(".defs.lua")
}
