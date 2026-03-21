use crate::{
    lua::{extract_lua_error, load_lua, setup_lua},
    registry::TestRegistry,
};

mod cli;
mod expect;
mod http;
mod lua;
mod registry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TestRegistry::new();
    let lua = setup_lua(registry.clone())?;
    let lua_content = load_lua("main.lua")?;
    lua.load(lua_content).exec()?;
    let loaded_tests = registry.get_tests();
    for test in loaded_tests {
        match test.func.call::<()>(()) {
            Ok(_) => println!("{:?}: pass", test.name),
            Err(e) => println!("{:?}: failed, {:?}", test.name, extract_lua_error(e)),
        }
    }
    Ok(())
}
