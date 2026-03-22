use futures::future::join_all;

use crate::{
    lua::{extract_lua_error, load_lua, setup_lua},
    registry::TestRegistry,
};

mod cli;
mod env;
mod expect;
mod http;
mod lua;
mod registry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TestRegistry::new();
    let lua = setup_lua(registry.clone())?;
    let lua_content = load_lua("main.lua")?;
    lua.load(lua_content).exec_async().await?;
    let loaded_tests = registry.get_tests();
    let results = join_all(loaded_tests.into_iter().map(|test| async move {
        let name = test.name;
        let result = test.func.call_async::<()>(()).await;
        (name, result)
    }))
    .await;

    for (name, result) in results {
        match result {
            Ok(_) => println!("{:?}: pass", name),
            Err(e) => println!("{:?}: failed, {:?}", name, extract_lua_error(e)),
        }
    }
    Ok(())
}
