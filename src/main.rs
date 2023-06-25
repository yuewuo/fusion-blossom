use clap::Parser;
use fusion_blossom::cli;

pub fn main() {
    cli::Cli::parse().run();
}
