use std::{collections::HashMap, str::FromStr, time::Duration};

use mlua::{Lua, Result, String as LuaString, Table, UserData, Value as LuaValue};
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::Value as JsonValue;

use crate::http::client::json_to_lua;

#[derive(Clone, Debug)]
pub struct RequestBuilder {
    pub method: Method,
    pub base_url: Option<String>,
    pub path: Option<String>,
    pub url: Option<String>,
    pub headers: HeaderMap<HeaderValue>,
    pub query: HashMap<String, String>,
    pub body: Option<Body>,
    pub json: Option<JsonValue>,
    pub timeout: Option<Duration>,
}

#[derive(Clone, Debug)]
pub enum Body {
    Text(String),
    Form(HashMap<String, String>),
    Bytes(Vec<u8>),
}

impl RequestBuilder {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            method: Method::GET,
            base_url,
            path: None,
            url: None,
            headers: HeaderMap::new(),
            query: HashMap::new(),
            body: None,
            json: None,
            timeout: None,
        }
    }

    fn build_url(&self) -> Result<String> {
        if let Some(url) = &self.url {
            reqwest::Url::parse(url).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "invalid url `{url}`: {e}. expected absolute URL like `https://example.com/path`"
                ))
            })?;
            return Ok(url.clone());
        }

        match (&self.base_url, &self.path) {
            (Some(base), Some(path)) => {
                let base_url = reqwest::Url::parse(base).map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "invalid base_url `{base}`: {e}. use something like `https://api.example.com`"
                    ))
                })?;
                let joined = base_url.join(path).map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "invalid path `{path}` for base_url `{base}`: {e}. \
use a leading slash path like `/users` or pass absolute URL via `:url(...)`"
                    ))
                })?;
                Ok(joined.to_string())
            }
            (None, Some(path)) => {
                if reqwest::Url::parse(path).is_ok() {
                    Ok(path.clone())
                } else {
                    Err(mlua::Error::RuntimeError(format!(
                        "relative path `{path}` used without base_url. \
set it with `client:base_url(\"https://api.example.com\")` \
or call `:url(\"https://api.example.com{path}\")`"
                    )))
                }
            }
            _ => Err(mlua::Error::RuntimeError(
                "missing request target: call `:url(...)` or `:path(...)`. \
if you use `:path(...)`, set `client:base_url(...)` first"
                    .into(),
            )),
        }
    }

    pub async fn execute(&self, lua: &Lua) -> Result<Table> {
        let client = Client::new();
        let start = std::time::Instant::now();
        let method = self.method.as_str().to_string();

        let url = self.build_url()?;
        let mut req = client.request(self.method.clone(), &url);
        req = req.headers(self.headers.clone());
        req = apply_query(req, &self.query);
        req = apply_body(req, self);
        if let Some(timeout) = self.timeout {
            req = req.timeout(timeout);
        }

        let res = req.send().await.map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "failed to send {method} request to `{url}`: {}",
                describe_reqwest_error(&e)
            ))
        })?;
        let response_url = res.url().to_string();
        let response_headers = res.headers().clone();
        let status = res.status().as_u16();
        let text = res.text().await.map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "failed to read response body for {method} `{url}`: {}",
                describe_reqwest_error(&e)
            ))
        })?;
        let duration_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);

        let json: Option<JsonValue> = serde_json::from_str(&text).ok();

        let table = lua.create_table()?;
        table.set("status", status)?;
        table.set("ok", (200..300).contains(&status))?;
        table.set("body", text.clone())?;
        table.set("url", response_url)?;
        table.set("duration_ms", duration_ms)?;
        table.set("headers", headers_to_lua(lua, &response_headers)?)?;
        if let Some(json) = json {
            let lua_json = json_to_lua(lua, json)?;
            table.set("json", lua_json)?;
        }
        Ok(table)
    }
}

impl UserData for RequestBuilder {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("path", |_, this, path: LuaValue| {
            let path = lua_value_to_string(path, "path passed to `:path(...)`")?;
            if path.trim().is_empty() {
                return Err(mlua::Error::RuntimeError(
                    "path passed to `:path(...)` cannot be empty".to_string(),
                ));
            }
            this.path = Some(path);
            Ok(this.clone())
        });
        methods.add_method_mut("url", |_, this, url: LuaValue| {
            let url = lua_value_to_string(url, "url passed to `:url(...)`")?;
            if url.trim().is_empty() {
                return Err(mlua::Error::RuntimeError(
                    "url passed to `:url(...)` cannot be empty".to_string(),
                ));
            }
            this.url = Some(url);
            Ok(this.clone())
        });
        methods.add_method_mut("header", |_, this, (k, v): (LuaValue, LuaValue)| {
            let key = lua_value_to_string(k, "header name passed to `:header(...)`")?;
            let value = lua_value_to_string(v, &format!("header value for `{key}`"))?;
            let (header_name, header_value) = parse_header(&key, &value)?;
            this.headers.insert(header_name, header_value);
            Ok(this.clone())
        });
        methods.add_method_mut("headers", |_, this, headers: Table| {
            for pair in headers.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair.map_err(|e| {
                    mlua::Error::RuntimeError(format!("invalid entry in `:headers(...)`: {e}"))
                })?;
                let key = lua_value_to_string(k, "header name in `:headers(...)`")?;
                let value = lua_value_to_string(v, &format!("header value for `{key}`"))?;
                let (header_name, header_value) = parse_header(&key, &value)?;
                this.headers.insert(header_name, header_value);
            }
            Ok(this.clone())
        });
        methods.add_method_mut("query", |_, this, (k, v): (LuaValue, LuaValue)| {
            let key = lua_value_to_string(k, "query key passed to `:query(...)`")?;
            let value = lua_value_to_string(v, &format!("query value for `{key}`"))?;
            this.query.insert(key, value);
            Ok(this.clone())
        });
        methods.add_method_mut("queries", |_, this, query: Table| {
            for pair in query.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair.map_err(|e| {
                    mlua::Error::RuntimeError(format!("invalid entry in `:queries(...)`: {e}"))
                })?;
                let key = lua_value_to_string(k, "query key in `:queries(...)`")?;
                let value = lua_value_to_string(v, &format!("query value for `{key}`"))?;
                this.query.insert(key, value);
            }
            Ok(this.clone())
        });
        methods.add_method_mut("body", |_, this, body: LuaValue| {
            let body = lua_value_to_string(body, "body passed to `:body(...)`")?;
            this.body = Some(Body::Text(body));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("body_bytes", |_, this, body: LuaString| {
            this.body = Some(Body::Bytes(body.as_bytes().to_vec()));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("json", |_, this, raw_json: LuaValue| {
            let raw_json = lua_value_to_string(raw_json, "json body passed to `:json(...)`")?;
            let parsed_json: JsonValue = serde_json::from_str(&raw_json).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "invalid json body: {e}. body preview: `{}`",
                    preview(&raw_json, 140)
                ))
            })?;
            this.json = Some(parsed_json);
            this.body = None;
            Ok(this.clone())
        });
        methods.add_method_mut("form", |_, this, form: Table| {
            let mut form_values = HashMap::new();
            for pair in form.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair.map_err(|e| {
                    mlua::Error::RuntimeError(format!("invalid entry in `:form(...)`: {e}"))
                })?;
                let key = lua_value_to_string(k, "form key in `:form(...)`")?;
                let value = lua_value_to_string(v, &format!("form value for `{key}`"))?;
                form_values.insert(key, value);
            }
            this.body = Some(Body::Form(form_values));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("timeout_ms", |_, this, timeout_ms: i64| {
            if timeout_ms <= 0 {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid timeout `{timeout_ms}` in `:timeout_ms(...)`: value must be greater than 0"
                )));
            }
            this.timeout = Some(Duration::from_millis(timeout_ms as u64));
            Ok(this.clone())
        });
        methods.add_method_mut("bearer", |_, this, token: LuaValue| {
            let token = lua_value_to_string(token, "token passed to `:bearer(...)`")?;
            if token.trim().is_empty() {
                return Err(mlua::Error::RuntimeError(
                    "token passed to `:bearer(...)` cannot be empty".to_string(),
                ));
            }
            let auth_value = format!("Bearer {token}");
            let header_value = HeaderValue::from_str(&auth_value).map_err(|e| {
                mlua::Error::RuntimeError(format!("invalid bearer token header value: {e}"))
            })?;
            this.headers
                .insert(HeaderName::from_static("authorization"), header_value);
            Ok(this.clone())
        });

        methods.add_async_method_mut("get", |lua, mut this, ()| async move {
            this.method = Method::GET;
            this.execute(&lua).await
        });
        methods.add_async_method_mut("post", |lua, mut this, ()| async move {
            this.method = Method::POST;
            this.execute(&lua).await
        });
        methods.add_async_method_mut("put", |lua, mut this, ()| async move {
            this.method = Method::PUT;
            this.execute(&lua).await
        });
        methods.add_async_method_mut("patch", |lua, mut this, ()| async move {
            this.method = Method::PATCH;
            this.execute(&lua).await
        });
        methods.add_async_method_mut("delete", |lua, mut this, ()| async move {
            this.method = Method::DELETE;
            this.execute(&lua).await
        });
        methods.add_async_method(
            "send",
            |lua, this, ()| async move { this.execute(&lua).await },
        );
    }
}

fn parse_header(key: &str, value: &str) -> Result<(HeaderName, HeaderValue)> {
    let header_name = HeaderName::from_str(key).map_err(|e| {
        mlua::Error::RuntimeError(format!(
            "invalid header name `{key}`: {e}. expected names like `content-type` or `x-api-key`"
        ))
    })?;
    let header_value = HeaderValue::from_str(value)
        .map_err(|e| mlua::Error::RuntimeError(format!("invalid header value for `{key}`: {e}")))?;
    Ok((header_name, header_value))
}

fn headers_to_lua(lua: &Lua, headers: &HeaderMap<HeaderValue>) -> Result<Table> {
    let table = lua.create_table()?;
    for (key, value) in headers {
        if let Ok(value_str) = value.to_str() {
            table.set(key.as_str(), value_str)?;
        }
    }
    Ok(table)
}

fn apply_query(
    req: reqwest::RequestBuilder,
    query: &HashMap<String, String>,
) -> reqwest::RequestBuilder {
    if query.is_empty() {
        req
    } else {
        req.query(query)
    }
}

fn apply_body(
    mut req: reqwest::RequestBuilder,
    builder: &RequestBuilder,
) -> reqwest::RequestBuilder {
    if let Some(json) = &builder.json {
        return req.json(json);
    }

    if let Some(body) = &builder.body {
        match body {
            Body::Text(s) => req = req.body(s.clone()),
            Body::Bytes(b) => req = req.body(b.clone()),
            Body::Form(f) => req = req.form(f),
        }
    }

    req
}

fn lua_value_to_string(value: LuaValue, field: &str) -> Result<String> {
    match value {
        LuaValue::String(s) => s
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| mlua::Error::RuntimeError(format!("{field} must be valid UTF-8: {e}"))),
        LuaValue::Integer(i) => Ok(i.to_string()),
        LuaValue::Number(n) => Ok(n.to_string()),
        LuaValue::Boolean(b) => Ok(b.to_string()),
        other => Err(mlua::Error::RuntimeError(format!(
            "{field} must be a string/number/boolean, got {}",
            other.type_name()
        ))),
    }
}

fn describe_reqwest_error(err: &reqwest::Error) -> String {
    let mut kind = Vec::new();
    if err.is_timeout() {
        kind.push("timeout");
    }
    if err.is_connect() {
        kind.push("connection error");
    }
    if err.is_request() {
        kind.push("request build error");
    }
    if err.is_body() {
        kind.push("request/response body error");
    }
    if err.is_decode() {
        kind.push("decode error");
    }

    if kind.is_empty() {
        err.to_string()
    } else {
        format!("{}: {}", kind.join(", "), err)
    }
}

fn preview(input: &str, max_chars: usize) -> String {
    let mut out: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
