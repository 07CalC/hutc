use clap::Parser;
use futures::future::join_all;
use std::time::Instant;

use crate::{
    cli::Args,
    fs::load_lua,
    init::init,
    lua::{extract_lua_error, setup_lua},
    registry::TestRegistry,
};

mod cli;
mod env;
mod expect;
mod fs;
mod http;
mod init;
mod lua;
mod registry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if &args.command == "init" {
        init(args.path)?;
    } else if &args.command == "test" {
        let suite_start = Instant::now();
        let registry = TestRegistry::new();
        let lua = setup_lua(registry.clone())?;
        let lua_files = load_lua(&args.path)?;
        for lua_content in lua_files {
            lua.load(lua_content).exec_async().await?;
        }
        let loaded_tests = registry.get_tests();
        let total = loaded_tests.len();

        println!("running {total} test{}", if total == 1 { "" } else { "s" });

        let results = join_all(loaded_tests.into_iter().map(|test| async move {
            let started_at = Instant::now();
            let name = test.name;
            let result = test.func.call_async::<()>(()).await;
            let duration_ms = started_at.elapsed().as_millis();
            (name, result, duration_ms)
        }))
        .await;

        let mut passed = 0usize;
        let mut failed = 0usize;

        for (index, (name, result, duration_ms)) in results.into_iter().enumerate() {
            match result {
                Ok(_) => {
                    passed += 1;
                    println!(
                        "[{}/{}] PASS {} ({} ms)",
                        index + 1,
                        total,
                        name,
                        duration_ms
                    );
                }
                Err(e) => {
                    failed += 1;
                    let err = extract_lua_error(e);
                    println!(
                        "[{}/{}] FAIL {} ({} ms)",
                        index + 1,
                        total,
                        name,
                        duration_ms
                    );
                    for line in err.lines() {
                        println!("  {line}");
                    }
                }
            }
        }

        let suite_duration_ms = suite_start.elapsed().as_millis();
        println!();
        println!(
            "summary: total={} passed={} failed={} duration={} ms",
            total, passed, failed, suite_duration_ms
        );
    }

    Ok(())
}
