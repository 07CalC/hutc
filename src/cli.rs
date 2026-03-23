use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "hutc")]
#[command(about = "HTTP API testing with Lua scripts")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Test {
        #[arg(default_value = "tests")]
        path: String,
    },
    Init {
        #[arg(default_value = "defs")]
        path: String,
    },
}
