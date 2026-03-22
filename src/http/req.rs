use std::{collections::HashMap, str::FromStr};

use mlua::{Lua, Result, Table, UserData};
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
    pub timeout: Option<std::time::Duration>,
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

        let url = self.build_url()?;
        let mut req = client.request(self.method.clone(), &url);
        req = req.headers(self.headers.clone());
        req = apply_query(req, &self.query);
        req = apply_body(req, self);

        let res = req
            .send()
            .await
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
        let status = res.status().as_u16();
        let text = res
            .text()
            .await
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

        let json: Option<Value> = serde_json::from_str(&text).ok();

        let table = lua.create_table()?;
        table.set("status", status)?;
        table.set("body", text.clone())?;
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
        methods.add_method_mut("header", |_, this, (k, v): (String, String)| {
            let header_name = HeaderName::from_str(&k).map_err(|e| {
                mlua::Error::RuntimeError(format!("invalid header name `{k}`: {e}"))
            })?;
            let header_value = HeaderValue::from_str(&v).map_err(|e| {
                mlua::Error::RuntimeError(format!("invalid header value for `{k}`: {e}"))
            })?;
            this.headers.insert(header_name, header_value);
            Ok(this.clone())
        });
        methods.add_method_mut("query", |_, this, (k, v): (String, String)| {
            this.query.insert(k, v);
            Ok(this.clone())
        });
        // methods.add_method_mut("json", |_, this, val: serde_json::Value| {
        //     this.json = Some(val);
        //     Ok(())
        // });

        methods.add_async_method_mut("get", |lua, mut this, ()| async move {
            this.method = Method::GET;
            this.execute(&lua).await
        });
        methods.add_async_method_mut("post", |lua, mut this, ()| async move {
            this.method = Method::POST;
            this.execute(&lua).await
        });
    }
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
