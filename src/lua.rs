use std::time::Duration;

use mlua::Variadic;
use mlua::prelude::*;
use tokio::time::sleep;

use crate::{env::Env, expect::Expect, http::client::HttpClient, registry::TestRegistry};

pub fn setup_lua(registry: TestRegistry) -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();

    let globals = lua.globals();

    let test_fn = lua.create_function(move |_, (name, func): (String, LuaFunction)| {
        if name.trim().is_empty() {
            return Err(mlua::Error::RuntimeError(
                "test name cannot be empty".to_string(),
            ));
        }
        registry.add_test(name, func);
        Ok(())
    })?;

    let expect_fn = lua.create_function(move |_, value: LuaValue| {
        Ok(Expect {
            value,
            error_message: None,
        })
    })?;

    let http_fn = lua.create_function(|_, ()| {
        let http_client = HttpClient::new();
        Ok(http_client)
    })?;

    let env_fn = lua.create_function(|_, path: Option<String>| {
        Env::load(path.unwrap_or_else(|| Env::default_path().to_string()))
            .map_err(|err| mlua::Error::RuntimeError(err.to_string()))
    })?;

    let log_fn = lua.create_function(|_, values: Variadic<LuaValue>| {
        let line = values
            .into_iter()
            .map(|value| format!("{value:#?}"))
            .collect::<Vec<_>>()
            .join("\t");
        println!("{line}");
        Ok(())
    })?;

    let sleep_fn = lua.create_async_function(|_, time: u64| async move {
        sleep(Duration::new(time, 0)).await;
        Ok(())
    })?;

    globals.set("http", http_fn)?;
    globals.set("env", env_fn)?;
    globals.set("test", test_fn)?;
    globals.set("expect", expect_fn)?;
    globals.set("log", log_fn)?;
    globals.set("sleep", sleep_fn)?;

    Ok(lua)
}

pub fn extract_lua_error(err: mlua::Error) -> String {
    let (message, location) = extract_error_parts(&err);
    match location {
        Some(loc) => format!("{loc}: {message}"),
        None => message,
    }
}

fn extract_error_parts(err: &mlua::Error) -> (String, Option<String>) {
    match err {
        mlua::Error::CallbackError { traceback, cause } => {
            let (message, _) = extract_error_parts(cause.as_ref());
            let location = extract_user_location(traceback);
            (message, location)
        }
        mlua::Error::RuntimeError(msg) => {
            // Runtime errors often include location in the message like:
            // "[string \"file.lua\"]:10: actual error message"
            if let Some((loc, msg)) = parse_lua_error_message(msg) {
                (msg, Some(loc))
            } else {
                (msg.clone(), None)
            }
        }
        other => (other.to_string(), None),
    }
}

/// Extract the first user-code location from a Lua traceback
fn extract_user_location(traceback: &str) -> Option<String> {
    for line in traceback.lines() {
        let line = line.trim();
        // Skip internal frames
        if line.is_empty()
            || line.starts_with("stack traceback:")
            || line.contains("[C]:")
            || line.contains("[string \"?\"]")
            || line.contains("src/main.rs")
        {
            continue;
        }
        // Extract location from lines like:
        // [string "tests/file.lua"]:10: in function <...>
        if let Some(loc) = parse_traceback_location(line) {
            return Some(loc);
        }
    }
    None
}

/// Parse location from a traceback line like:
/// `[string "tests/file.lua"]:10: in function <...>`
/// Returns: `tests/file.lua:10`
fn parse_traceback_location(line: &str) -> Option<String> {
    let line = line.trim_start_matches("[string \"");
    let end_quote = line.find("\"]:")?;
    let file = &line[..end_quote];
    let rest = &line[end_quote + 3..];
    let line_end = rest.find(':').unwrap_or(rest.len());
    let line_num = rest[..line_end].trim();
    if line_num.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{file}:{line_num}"))
    } else {
        Some(file.to_string())
    }
}

/// Parse Lua error messages that include location like:
/// `[string "file.lua"]:10: attempt to index nil`
/// Returns: (location, message)
fn parse_lua_error_message(msg: &str) -> Option<(String, String)> {
    if !msg.starts_with("[string \"") {
        return None;
    }
    let after_prefix = &msg[9..]; // skip `[string "`
    let end_quote = after_prefix.find("\"]:")?;
    let file = &after_prefix[..end_quote];
    let rest = &after_prefix[end_quote + 3..]; // skip `"]:`

    // Find line number and message
    let colon = rest.find(':')?;
    let line_num = rest[..colon].trim();
    let mut message = rest[colon + 1..].trim();

    // Strip stack traceback if present
    if let Some(traceback_start) = message.find("\nstack traceback:") {
        message = message[..traceback_start].trim();
    }

    if line_num.chars().all(|c| c.is_ascii_digit()) {
        Some((format!("{file}:{line_num}"), message.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::setup_lua;
    use crate::registry::TestRegistry;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::runtime::Runtime;

    #[test]
    fn setup_lua_registers_env_helper() {
        let env_path = std::env::temp_dir().join(format!(
            "hutc-lua-env-{}.env",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&env_path, "TOKEN=from_file\n").unwrap();

        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            let lua = setup_lua(TestRegistry::new()).unwrap();
            let token: String = lua
                .load(format!(
                    r#"
                    local env_file = env("{}")
                    return env_file:require("TOKEN")
                    "#,
                    env_path.display()
                ))
                .eval_async()
                .await
                .unwrap();
            assert_eq!(token, "from_file");
        });

        let _ = fs::remove_file(env_path);
    }
}
