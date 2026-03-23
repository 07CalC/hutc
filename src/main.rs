use clap::Parser;
use futures::future::join_all;
use std::{
    io::{Error, ErrorKind},
    time::Instant,
};

use crate::{
    cli::{Cli, Command},
    fs::load_lua,
    init::init,
    lua::{extract_lua_error, setup_lua},
    registry::TestRegistry,
};

mod cli;
mod expect;
mod fs;
mod http;
mod init;
mod lua;
mod registry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { path } => init(path)?,
        Command::Test { path } => run_tests(&path).await?,
    }

    Ok(())
}

async fn run_tests(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let suite_start = Instant::now();
    let registry = TestRegistry::new();
    let lua = setup_lua(registry.clone())?;
    let lua_files = load_lua(path)?;

    for lua_file in lua_files {
        let chunk_name = lua_file.path.to_string_lossy().to_string();
        let chunk = lua.load(&lua_file.content).set_name(&chunk_name);
        if let Err(err) = chunk.exec_async().await {
            let message = extract_lua_error(err);
            return Err(Error::other(format!(
                "failed to execute Lua file `{}`:\n{}",
                lua_file.path.display(),
                message
            ))
            .into());
        }
    }

    let loaded_tests = registry.get_tests();
    let total = loaded_tests.len();
    if total == 0 {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "no tests were registered from `{path}`. make sure files call `test(\"name\", fn)`"
            ),
        )
        .into());
    }

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
    let mut failed_tests = Vec::new();

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
                failed_tests.push(name.clone());
                let err = extract_lua_error(e);
                println!(
                    "[{}/{}] FAIL {} ({} ms)",
                    index + 1,
                    total,
                    name,
                    duration_ms
                );
                println!("  error:");
                for line in err.lines() {
                    println!("    {line}");
                }
                println!("  end error");
            }
        }
    }

    let suite_duration_ms = suite_start.elapsed().as_millis();
    println!();
    println!(
        "summary: total={} passed={} failed={} duration={} ms",
        total, passed, failed, suite_duration_ms
    );
    if !failed_tests.is_empty() {
        println!("failed tests:");
        for name in failed_tests {
            println!("  - {name}");
        }
        return Err(Error::other(format!("{failed}/{total} test(s) failed")).into());
    }

    Ok(())
}
