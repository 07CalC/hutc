use mlua::{UserData, Value};

#[derive(Clone)]
pub struct Expect {
    pub value: Value,
}

impl UserData for Expect {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("to_equal", |_, this, expected: Value| {
            if this.value == expected {
                Ok(())
            } else {
                Err(mlua::Error::RuntimeError(format!(
                    "expected {:?} got {:?}",
                    expected, this.value
                )))
            }
        });
        methods.add_method("to_not_equal", |_, this, expected: Value| {
            if this.value != expected {
                Ok(())
            } else {
                Err(mlua::Error::RuntimeError(format!(
                    "did not expect {:?}",
                    expected
                )))
            }
        });
        methods.add_method("to_exist", |_, this, _: ()| match this.value {
            Value::Nil => Err(mlua::Error::RuntimeError(format!(
                "expected value to exist got nil"
            ))),
            _ => Ok(()),
        });
    }
}
