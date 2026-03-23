use std::{fs, io::Error, path::Path};

pub fn load_lua(path: &str) -> Result<Vec<String>, Error> {
    let path = Path::new(path);
    let mut out = Vec::new();
    if path.is_file() {
        let content = fs::read_to_string(path)?;
        out.push(content);
    } else {
        for entry in fs::read_dir(path)? {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let content = fs::read_to_string(path)?;
                out.push(content);
            }
        }
    }

    Ok(out)
}
