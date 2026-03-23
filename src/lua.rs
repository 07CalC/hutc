use mlua::Variadic;
use mlua::prelude::*;

use crate::{expect::Expect, http::client::HttpClient, registry::TestRegistry};

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

    let log_fn = lua.create_function(|_, values: Variadic<LuaValue>| {
        let line = values
            .into_iter()
            .map(|value| format!("{value:#?}"))
            .collect::<Vec<_>>()
            .join("\t");
        println!("{line}");
        Ok(())
    })?;

    globals.set("http", http_fn)?;
    globals.set("test", test_fn)?;
    globals.set("expect", expect_fn)?;
    globals.set("log", log_fn)?;

    Ok(lua)
}

pub fn extract_lua_error(err: mlua::Error) -> String {
    let mut lines = Vec::new();
    collect_error_lines(&err, &mut lines, 0);
    if lines.is_empty() {
        "<unknown Lua error>".to_string()
    } else {
        lines.join("\n")
    }
}

fn collect_error_lines(err: &mlua::Error, lines: &mut Vec<String>, depth: usize) {
    let indent = "  ".repeat(depth);
    match err {
        mlua::Error::CallbackError { traceback, cause } => {
            lines.push(format!("{indent}lua callback error"));

            let traceback = traceback.trim();
            if !traceback.is_empty() {
                lines.push(format!("{indent}traceback:"));
                for line in traceback.lines() {
                    let line = line.trim_end();
                    if !line.is_empty() && !line.contains("src/main.rs:") {
                        lines.push(format!("{indent}  {line}"));
                    }
                }
            }

            lines.push(format!("{indent}cause:"));
            collect_error_lines(cause.as_ref(), lines, depth + 1);
        }
        other => {
            let message = other.to_string();
            if message.trim().is_empty() {
                lines.push(format!("{indent}<empty error>"));
            } else {
                for line in message.lines() {
                    lines.push(format!("{indent}{line}"));
                }
            }
        }
    }
}
