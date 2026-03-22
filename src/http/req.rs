use std::{collections::HashMap, str::FromStr, time::Duration};

use mlua::{Lua, Result, String as LuaString, Table, UserData};
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::Value;

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
    pub json: Option<Value>,
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
            return Ok(url.clone());
        }
        match (&self.base_url, &self.path) {
            (Some(base), Some(path)) => Ok(format!("{}{}", base, path)),
            (_, Some(path)) => Ok(path.clone()),
            _ => Err(mlua::Error::RuntimeError("No URL or path provided".into())),
        }
    }
    pub async fn execute(&self, lua: &Lua) -> Result<Table> {
        let client = Client::new();
        let start = std::time::Instant::now();

        let url = self.build_url()?;
        let mut req = client.request(self.method.clone(), &url);
        req = req.headers(self.headers.clone());
        req = apply_query(req, &self.query);
        req = apply_body(req, self);
        if let Some(timeout) = self.timeout {
            req = req.timeout(timeout);
        }

        let res = req
            .send()
            .await
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
        let response_url = res.url().to_string();
        let response_headers = res.headers().clone();
        let status = res.status().as_u16();
        let text = res
            .text()
            .await
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
        let duration_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);

        let json: Option<Value> = serde_json::from_str(&text).ok();

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
        methods.add_method_mut("path", |_, this, path: String| {
            this.path = Some(path);
            Ok(this.clone())
        });
        methods.add_method_mut("url", |_, this, url: String| {
            this.url = Some(url);
            Ok(this.clone())
        });
        methods.add_method_mut("header", |_, this, (k, v): (String, String)| {
            let (header_name, header_value) = parse_header(&k, &v)?;
            this.headers.insert(header_name, header_value);
            Ok(this.clone())
        });
        methods.add_method_mut("headers", |_, this, headers: Table| {
            for pair in headers.pairs::<String, String>() {
                let (k, v) = pair?;
                let (header_name, header_value) = parse_header(&k, &v)?;
                this.headers.insert(header_name, header_value);
            }
            Ok(this.clone())
        });
        methods.add_method_mut("query", |_, this, (k, v): (String, String)| {
            this.query.insert(k, v);
            Ok(this.clone())
        });
        methods.add_method_mut("queries", |_, this, query: Table| {
            for pair in query.pairs::<String, String>() {
                let (k, v) = pair?;
                this.query.insert(k, v);
            }
            Ok(this.clone())
        });
        methods.add_method_mut("body", |_, this, body: String| {
            this.body = Some(Body::Text(body));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("body_bytes", |_, this, body: LuaString| {
            this.body = Some(Body::Bytes(body.as_bytes().to_vec()));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("json", |_, this, raw_json: String| {
            let parsed_json: Value = serde_json::from_str(&raw_json)
                .map_err(|e| mlua::Error::RuntimeError(format!("invalid json body: {e}")))?;
            this.json = Some(parsed_json);
            this.body = None;
            Ok(this.clone())
        });
        methods.add_method_mut("form", |_, this, form: Table| {
            let mut form_values = HashMap::new();
            for pair in form.pairs::<String, String>() {
                let (k, v) = pair?;
                form_values.insert(k, v);
            }
            this.body = Some(Body::Form(form_values));
            this.json = None;
            Ok(this.clone())
        });
        methods.add_method_mut("timeout_ms", |_, this, timeout_ms: u64| {
            this.timeout = Some(Duration::from_millis(timeout_ms));
            Ok(this.clone())
        });
        methods.add_method_mut("bearer", |_, this, token: String| {
            let auth_value = format!("Bearer {token}");
            let header_value = HeaderValue::from_str(&auth_value).map_err(|e| {
                mlua::Error::RuntimeError(format!("invalid bearer token header value: {e}"))
            })?;
            this.headers.insert(HeaderName::from_static("authorization"), header_value);
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
        methods.add_async_method("send", |lua, this, ()| async move { this.execute(&lua).await });
    }
}

fn parse_header(key: &str, value: &str) -> Result<(HeaderName, HeaderValue)> {
    let header_name = HeaderName::from_str(key)
        .map_err(|e| mlua::Error::RuntimeError(format!("invalid header name `{key}`: {e}")))?;
    let header_value = HeaderValue::from_str(value).map_err(|e| {
        mlua::Error::RuntimeError(format!("invalid header value for `{key}`: {e}"))
    })?;
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
