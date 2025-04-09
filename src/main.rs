use anyhow::Result;
use std::env;

use cmd_thing::*;

fn main() -> Result<()> {
    // e.g. cargo run -- "generate ./data/wiki.bpe join these please -f 2 --max-tokens 30"
    let command_str = env::args().skip(1).next().unwrap();

    let mut cmd = Command::parse(command_str)?;

    let c = cmd.get_next_argument()?;
    let bpe_file = cmd.get_next_argument()?;
    let rest = cmd.drain_arguments().make_contiguous().join(" ");

    let idx_weight = Flag::new("idx-weight")
        .alias("i")
        .default(1.0)
        .parse(&mut cmd)?;
    let freq_weight = Flag::new("freq-weight")
        .alias("f")
        .default(1.0)
        .parse(&mut cmd)?;
    let max_tokens = Flag::new("max-tokens")
        .alias("t")
        .default(20)
        .parse(&mut cmd)?;

    println!("cmd:      {}", c);
    println!("bpe-file: {}", bpe_file);
    println!("rest:     {}", rest);
    println!("-i:       {}", idx_weight);
    println!("-f:       {}", freq_weight);
    println!("-t:       {}", max_tokens);

    Ok(())
}
