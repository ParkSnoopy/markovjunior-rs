//! Runs a two-dimensional `MarkovJunior` model from an XML file.

use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::PathBuf,
};

use rand::{SeedableRng, rngs::StdRng};
use markovjunior_rs::Model;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    let Some(model_path) = arguments.next() else {
        return Err(
            "usage: markovjunior-rs MODEL.xml [--width N] [--height N] [--steps N] [--seed N]"
                .into(),
        );
    };
    let mut width = 64;
    let mut height = 64;
    let mut steps = 10_000;
    let mut seed = 0_u64;
    while let Some(option) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option.as_str() {
            "--width" => width = value.parse()?,
            "--height" => height = value.parse()?,
            "--steps" => steps = value.parse()?,
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {option}").into()),
        }
    }
    let path = PathBuf::from(model_path);
    let xml = fs::read_to_string(&path)?;
    let mut model = Model::from_xml(&xml, width, height)?;
    let mut rng = StdRng::seed_from_u64(seed);
    let completed = model.run(steps, &mut rng);
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(output, "steps: {completed}")?;
    for row in model.grid().rows() {
        writeln!(output, "{row}")?;
    }
    Ok(())
}
