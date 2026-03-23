use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    pub command: String,

    pub path: String,
}

