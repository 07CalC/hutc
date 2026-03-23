use mlua::{Lua, UserData};
use reqwest::Url;
use serde_json::Value as JsonValue;

use crate::http::req::RequestBuilder;

pub struct HttpClient {
    pub base_url: Option<String>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self { base_url: None }
    }
}

impl UserData for HttpClient {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("base_url", |_, this, url: String| {
            let parsed = Url::parse(&url).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "invalid base_url `{url}`: {e}. expected absolute URL like `https://api.example.com`"
                ))
            })?;

            let scheme = parsed.scheme();
            if scheme != "http" && scheme != "https" {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid base_url `{url}`: unsupported scheme `{scheme}`. only `http` and `https` are allowed"
                )));
            }

            if parsed.host_str().is_none() {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid base_url `{url}`: host is missing"
                )));
            }

            this.base_url = Some(parsed.to_string());
            Ok(())
        });
        methods.add_method("req", |_, this, ()| {
            Ok(RequestBuilder::new(this.base_url.clone()))
        });
        // methods.add_async_method("get", |lua, this, path: String| async move {
        //     let url = this.build_url(path);
        //
        //     let client = reqwest::Client::new();
        //
        //     let response = client
        //         .get(&url)
        //         .send()
        //         .await
        //         .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
        //
        //     let status = response.status().as_u16();
        //     let text = response
        //         .text()
        //         .await
        //         .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
        //
        //     let json: Option<JsonValue> = serde_json::from_str(&text).ok();
        //
        //     let res_table = lua.create_table()?;
        //     res_table.set("status", status)?;
        //     res_table.set("body", text)?;
        //
        //     if let Some(json_val) = json {
        //         let lua_json = json_to_lua(&lua, json_val)?;
        //         res_table.set("json", lua_json.clone())?;
        //     }
        //     Ok(res_table)
        // });
    }
}

pub fn json_to_lua(lua: &Lua, value: JsonValue) -> Result<mlua::Value, mlua::Error> {
    Ok(match value {
        JsonValue::Null => mlua::Value::Nil,
        JsonValue::Bool(b) => mlua::Value::Boolean(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                mlua::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                mlua::Value::Number(f)
            } else {
                mlua::Value::Nil
            }
        }
        JsonValue::String(s) => mlua::Value::String(lua.create_string(&s)?),

        JsonValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.into_iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, v)?)?;
            }
            mlua::Value::Table(table)
        }

        JsonValue::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj {
                table.set(k, json_to_lua(lua, v)?)?;
            }
            mlua::Value::Table(table)
        }
    })
}
