use mlua::{UserData, Value};

#[derive(Clone)]
pub struct Expect {
    pub value: Value,
    pub error_message: Option<String>,
}

impl Expect {
    fn assertion_error(&self, detail: String) -> mlua::Error {
        if let Some(msg) = &self.error_message {
            mlua::Error::RuntimeError(format!("{msg}: {detail}"))
        } else {
            mlua::Error::RuntimeError(detail)
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => match s.to_str() {
            Ok(s) => format!("\"{s}\""),
            Err(_) => "<invalid utf8>".to_string(),
        },
        Value::Table(_) => "<table>".to_string(),
        Value::Function(_) => "<function>".to_string(),
        Value::Thread(_) => "<thread>".to_string(),
        Value::UserData(_) => "<userdata>".to_string(),
        Value::LightUserData(_) => "<lightuserdata>".to_string(),
        Value::Error(e) => format!("<error: {e}>"),
        Value::Other(..) => "<other>".to_string(),
    }
}

impl UserData for Expect {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("msg", |_, this, message: String| {
            this.error_message = Some(message);
            Ok(this.clone())
        });
        methods.add_method("to_equal", |_, this, expected: Value| {
            if this.value == expected {
                Ok(())
            } else {
                Err(this.assertion_error(format!(
                    "expected {} but got {}",
                    format_value(&expected),
                    format_value(&this.value)
                )))
            }
        });
        methods.add_method("to_not_equal", |_, this, expected: Value| {
            if this.value != expected {
                Ok(())
            } else {
                Err(this.assertion_error(format!(
                    "expected value to not equal {}",
                    format_value(&expected)
                )))
            }
        });
        methods.add_method("to_exist", |_, this, _: ()| match this.value {
            Value::Nil => Err(this.assertion_error("expected value to exist but got nil".into())),
            _ => Ok(()),
        });
    }
}
