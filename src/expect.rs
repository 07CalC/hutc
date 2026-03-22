use mlua::{UserData, Value};

#[derive(Clone)]
pub struct Expect {
    pub value: Value,
    pub error_message: Option<String>,
}

impl Expect {
    fn assertion_error(&self, detail: String) -> mlua::Error {
        if let Some(msg) = &self.error_message {
            println!("{msg}");
            mlua::Error::RuntimeError(format!("{msg}: {detail}"))
        } else {
            mlua::Error::RuntimeError(detail)
        }
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
                    "expected {:?} got {:?}",
                    expected, this.value
                )))
            }
        });
        methods.add_method("to_not_equal", |_, this, expected: Value| {
            if this.value != expected {
                Ok(())
            } else {
                Err(this.assertion_error(format!("did not expect {:?}", expected)))
            }
        });
        methods.add_method("to_exist", |_, this, _: ()| match this.value {
            Value::Nil => Err(this.assertion_error("expected value to exist got nil".into())),
            _ => Ok(()),
        });
    }
}
