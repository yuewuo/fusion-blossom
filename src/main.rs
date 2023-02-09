use fusion_blossom::cli;
use clap::Parser;


pub fn main() {

    cli::Cli::parse().run();

}
