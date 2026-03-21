use std::sync::{Arc, Mutex};

use mlua::Function;

#[derive(Clone)]
pub struct Test {
    pub name: String,
    pub func: Function,
}

#[derive(Clone)]
pub struct TestRegistry {
    pub tests: Arc<Mutex<Vec<Test>>>,
}

impl TestRegistry {
    pub fn new() -> Self {
        Self {
            tests: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn add_test(&self, name: String, func: Function) {
        self.tests.lock().unwrap().push(Test { name, func });
    }

    pub fn get_tests(&self) -> Vec<Test> {
        self.tests.lock().unwrap().clone()
    }
}
