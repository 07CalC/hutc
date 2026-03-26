use std::{
    collections::HashMap,
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

use mlua::{Table, UserData};

const DEFAULT_ENV_PATH: &str = ".env";

#[derive(Debug, Clone)]
pub struct Env {
    pub path: PathBuf,
    pub envs: HashMap<String, String>,
}

impl Env {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, EnvLoadError> {
        let path = path.into();
        let envs = load_env_file(&path)?;
        Ok(Self { path, envs })
    }

    pub fn default_path() -> &'static str {
        DEFAULT_ENV_PATH
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.envs.get(key).cloned().or_else(|| env::var(key).ok())
    }

    pub fn require(&self, key: &str) -> Result<String, EnvLoadError> {
        self.get(key).ok_or_else(|| EnvLoadError::MissingKey {
            path: self.path.clone(),
            key: key.to_string(),
        })
    }

    pub fn reload(&mut self) -> Result<(), EnvLoadError> {
        self.envs = load_env_file(&self.path)?;
        Ok(())
    }

    pub fn to_lua_table(&self, lua: &mlua::Lua) -> mlua::Result<Table> {
        let table = lua.create_table()?;
        for (key, value) in &self.envs {
            table.set(key.as_str(), value.as_str())?;
        }
        Ok(table)
    }
}

impl UserData for Env {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "get",
            |_, this, (key, default): (String, Option<String>)| Ok(this.get(&key).or(default)),
        );
        methods.add_method("require", |_, this, key: String| {
            this.require(&key)
                .map_err(|err| mlua::Error::RuntimeError(err.to_string()))
        });
        methods.add_method_mut("reload", |_, this, ()| {
            this.reload()
                .map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
            Ok(this.clone())
        });
        methods.add_method("all", |lua, this, ()| this.to_lua_table(lua));
    }
}

#[derive(Debug)]
pub enum EnvLoadError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },
    MissingKey {
        path: PathBuf,
        key: String,
    },
}

impl fmt::Display for EnvLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read env file `{}`: {source}", path.display())
            }
            Self::Parse {
                path,
                line,
                message,
            } => write!(
                f,
                "failed to parse env file `{}` at line {}: {}",
                path.display(),
                line,
                message
            ),
            Self::MissingKey { path, key } => write!(
                f,
                "missing environment variable `{key}` in `{}` or the process environment",
                path.display()
            ),
        }
    }
}

impl std::error::Error for EnvLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { .. } | Self::MissingKey { .. } => None,
        }
    }
}

fn load_env_file(path: &Path) -> Result<HashMap<String, String>, EnvLoadError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(err) => {
            return Err(EnvLoadError::Io {
                path: path.to_path_buf(),
                source: err,
            });
        }
    };

    parse_env(&content).map_err(|err| EnvLoadError::Parse {
        path: path.to_path_buf(),
        line: err.line,
        message: err.message,
    })
}

fn parse_env(content: &str) -> Result<HashMap<String, String>, ParseError> {
    let mut parser = EnvParser::new(content);
    parser.parse()
}

#[derive(Debug, PartialEq, Eq)]
struct ParseError {
    line: usize,
    message: String,
}

struct EnvParser<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> EnvParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.strip_prefix('\u{feff}').unwrap_or(input),
            pos: 0,
            line: 1,
        }
    }

    fn parse(&mut self) -> Result<HashMap<String, String>, ParseError> {
        let mut out = HashMap::new();

        while !self.is_eof() {
            self.skip_leading_space();

            if self.is_eof() {
                break;
            }

            if self.consume_if('\n') {
                continue;
            }

            if self.peek_char() == Some('#') {
                self.skip_comment();
                continue;
            }

            self.consume_export_prefix();
            let key = self.parse_key()?;
            self.skip_inline_space();
            self.expect_char('=')?;
            self.skip_inline_space();
            let value = self.parse_value()?;
            self.skip_inline_space();

            if self.peek_char() == Some('#') {
                self.skip_comment();
            }

            match self.peek_char() {
                Some('\n') => {
                    self.advance_char();
                }
                None => {}
                Some(other) => {
                    return Err(self.error(format!(
                        "unexpected character `{other}` after value for `{key}`"
                    )));
                }
            }

            out.insert(key, value);
        }

        Ok(out)
    }

    fn parse_key(&mut self) -> Result<String, ParseError> {
        let mut key = String::new();
        let Some(first) = self.peek_char() else {
            return Err(self.error("expected environment variable name".to_string()));
        };

        if !is_valid_key_start(first) {
            return Err(self.error(format!("invalid environment variable name start `{first}`")));
        }

        key.push(self.advance_char().expect("peeked char should exist"));

        while let Some(ch) = self.peek_char() {
            if is_valid_key_char(ch) {
                key.push(self.advance_char().expect("peeked char should exist"));
            } else {
                break;
            }
        }

        Ok(key)
    }

    fn parse_value(&mut self) -> Result<String, ParseError> {
        match self.peek_char() {
            Some('"') => self.parse_double_quoted_value(),
            Some('\'') => self.parse_single_quoted_value(),
            _ => Ok(self.parse_unquoted_value()),
        }
    }

    fn parse_single_quoted_value(&mut self) -> Result<String, ParseError> {
        self.expect_char('\'')?;
        let mut value = String::new();

        loop {
            match self.advance_char() {
                Some('\'') => return Ok(value),
                Some(ch) => value.push(ch),
                None => {
                    return Err(self.error("unterminated single-quoted value".to_string()));
                }
            }
        }
    }

    fn parse_double_quoted_value(&mut self) -> Result<String, ParseError> {
        self.expect_char('"')?;
        let mut value = String::new();

        loop {
            match self.advance_char() {
                Some('"') => return Ok(value),
                Some('\\') => {
                    let escaped = match self.advance_char() {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('"') => '"',
                        Some('\\') => '\\',
                        Some('$') => '$',
                        Some(other) => other,
                        None => {
                            return Err(self.error(
                                "unterminated escape sequence in double-quoted value".to_string(),
                            ));
                        }
                    };
                    value.push(escaped);
                }
                Some(ch) => value.push(ch),
                None => {
                    return Err(self.error("unterminated double-quoted value".to_string()));
                }
            }
        }
    }

    fn parse_unquoted_value(&mut self) -> String {
        let mut value = String::new();
        let mut prev_was_whitespace = true;

        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }

            if ch == '#' && prev_was_whitespace {
                break;
            }

            if ch == '\\' {
                self.advance_char();
                if let Some(escaped) = self.peek_char() {
                    if escaped == '\n' {
                        break;
                    }
                    value.push(self.advance_char().expect("peeked char should exist"));
                    prev_was_whitespace = escaped.is_whitespace();
                } else {
                    value.push('\\');
                }
                continue;
            }

            value.push(self.advance_char().expect("peeked char should exist"));
            prev_was_whitespace = ch.is_whitespace();
        }

        value.trim_end().to_string()
    }

    fn consume_export_prefix(&mut self) {
        if !self.remaining().starts_with("export") {
            return;
        }

        let next = self.remaining()["export".len()..].chars().next();
        if matches!(next, Some(' ' | '\t' | '\r')) {
            for _ in 0.."export".len() {
                self.advance_char();
            }
            self.skip_inline_space();
        }
    }

    fn skip_leading_space(&mut self) {
        while matches!(self.peek_char(), Some(' ' | '\t' | '\r')) {
            self.advance_char();
        }
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek_char(), Some(' ' | '\t' | '\r')) {
            self.advance_char();
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.advance_char();
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        match self.advance_char() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(self.error(format!("expected `{expected}`, found `{ch}`"))),
            None => Err(self.error(format!("expected `{expected}`, found end of file"))),
        }
    }

    fn error(&self, message: String) -> ParseError {
        ParseError {
            line: self.line,
            message,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance_char();
            true
        } else {
            false
        }
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
        }
        Some(ch)
    }
}

fn is_valid_key_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_valid_key_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{Env, ParseError, parse_env};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parse_env_supports_comments_whitespace_and_export() {
        let envs = parse_env(
            r#"
            # comment
            export API_URL = https://example.com
            TOKEN=abc123 # inline comment
            PORT = 8080
            EMPTY=
            HASH=literal\#value
            "#,
        )
        .unwrap();

        assert_eq!(
            envs.get("API_URL"),
            Some(&"https://example.com".to_string())
        );
        assert_eq!(envs.get("TOKEN"), Some(&"abc123".to_string()));
        assert_eq!(envs.get("PORT"), Some(&"8080".to_string()));
        assert_eq!(envs.get("EMPTY"), Some(&String::new()));
        assert_eq!(envs.get("HASH"), Some(&"literal#value".to_string()));
    }

    #[test]
    fn parse_env_supports_quoted_values_and_escapes() {
        let envs =
            parse_env("SINGLE='a # literal'\nDOUBLE=\"line\\nvalue\"\nMULTI=\"hello\nworld\"\n")
                .unwrap();

        assert_eq!(envs.get("SINGLE"), Some(&"a # literal".to_string()));
        assert_eq!(envs.get("DOUBLE"), Some(&"line\nvalue".to_string()));
        assert_eq!(envs.get("MULTI"), Some(&"hello\nworld".to_string()));
    }

    #[test]
    fn parse_env_rejects_invalid_lines_with_line_number() {
        let err = parse_env("GOOD=value\n1BAD=value\n").unwrap_err();

        assert_eq!(
            err,
            ParseError {
                line: 2,
                message: "invalid environment variable name start `1`".to_string(),
            }
        );
    }

    #[test]
    fn env_loads_missing_files_as_empty() {
        let path = unique_temp_path("missing");
        let env = Env::load(&path).unwrap();

        assert!(env.envs.is_empty());
        assert_eq!(env.path, path);
    }

    #[test]
    fn env_reload_refreshes_values() {
        let path = unique_temp_path("reload");
        fs::write(&path, "TOKEN=first\n").unwrap();

        let mut env = Env::load(&path).unwrap();
        assert_eq!(env.get("TOKEN").as_deref(), Some("first"));

        fs::write(&path, "TOKEN=second\n").unwrap();
        env.reload().unwrap();

        assert_eq!(env.get("TOKEN").as_deref(), Some("second"));
        let _ = fs::remove_file(path);
    }

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hutc-{prefix}-{nanos}.env"))
    }
}
