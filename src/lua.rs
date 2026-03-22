use std::{fs, io::Error};

use mlua::prelude::*;

use crate::{expect::Expect, http::client::HttpClient, registry::TestRegistry};

pub fn load_lua(path: &str) -> Result<String, Error> {
    let lua_content = fs::read_to_string(path)?;
    Ok(lua_content)
}

pub fn setup_lua(registry: TestRegistry) -> Result<Lua, Box<dyn std::error::Error>> {
    let lua = Lua::new();

    let globals = lua.globals();

    let test_fn = lua.create_function(move |_, (name, func): (String, LuaFunction)| {
        registry.add_test(name, func);
        Ok(())
    })?;

    let expect_fn = lua.create_function(move |_, value: LuaValue| Ok(Expect { value }))?;

    let http_fn = lua.create_function(|_, ()| {
        let http_client = HttpClient::new();
        Ok(http_client)
    })?;

    globals.set("http", http_fn)?;
    globals.set("test", test_fn)?;
    globals.set("expect", expect_fn)?;
    Ok(lua)
}

pub fn extract_lua_error(err: mlua::Error) -> String {
    match err {
        mlua::Error::RuntimeError(msg) => msg,
        mlua::Error::CallbackError { cause, .. } => extract_lua_error((*cause).clone()),
        other => other.to_string(),
    }
}
