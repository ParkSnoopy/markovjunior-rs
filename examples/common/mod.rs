use std::{error::Error, io::Write, path::PathBuf};

pub fn gallery_options() -> Result<(PathBuf, u64), Box<dyn Error>> {
    let mut directory = None;
    let mut seed = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--seed" && seed.is_none() {
            let value = args.next().ok_or("missing value for --seed")?;
            seed = Some(value.to_str().ok_or("invalid seed")?.parse()?);
        } else if !arg.to_string_lossy().starts_with('-') && directory.is_none() {
            directory = Some(PathBuf::from(arg));
        } else {
            return Err("expected [DIRECTORY] [--seed N]".into());
        }
    }
    let seed = seed.unwrap_or_else(rand::random);
    writeln!(std::io::stderr(), "seed: {seed}")?;
    Ok((directory.unwrap_or_else(|| PathBuf::from("assets")), seed))
}
