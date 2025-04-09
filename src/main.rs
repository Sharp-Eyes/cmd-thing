use anyhow::Result;
use std::env;

use cmd_thing::*;

fn main() -> Result<()> {
    // e.g. cargo run -- "foo 11"
    let command_str = env::args().skip(1).next().unwrap();

    let mut cmd = Command::parse(command_str)?;

    let c: String = cmd.next_argument()?;
    let i = cmd.next_flag("idx_weight").unwrap_or(12);

    println!("cmd: {:?}", c);
    println!("-i: {:?}", i);

    Ok(())
}
