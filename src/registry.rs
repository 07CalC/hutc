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
        let mut guard = self.tests.lock().unwrap_or_else(|poisoned| {
            eprintln!(
                "warning: test registry lock was poisoned; recovering to preserve collected tests"
            );
            poisoned.into_inner()
        });
        guard.push(Test { name, func });
    }

    pub fn get_tests(&self) -> Vec<Test> {
        let guard = self.tests.lock().unwrap_or_else(|poisoned| {
            eprintln!("warning: test registry lock was poisoned; using recovered test list");
            poisoned.into_inner()
        });
        guard.clone()
    }
}
