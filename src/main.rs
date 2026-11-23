//! Runs a `MarkovJunior` XML model and exports text or voxels.

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
            "usage: markovjunior-rs MODEL.xml [--width N] [--height N] [--depth N] [--steps N] [--seed N] [--vox FILE]"
                .into(),
        );
    };
    let mut width = 64;
    let mut height = 64;
    let mut depth = 1;
    let mut vox = None;
    let mut steps = 10_000;
    let mut seed = None;
    while let Some(option) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option.as_str() {
            "--width" => width = value.parse()?,
            "--height" => height = value.parse()?,
            "--depth" => depth = value.parse()?,
            "--vox" => vox = Some(PathBuf::from(value)),
            "--steps" => steps = value.parse()?,
            "--seed" => seed = Some(value.parse()?),
            _ => return Err(format!("unknown option {option}").into()),
        }
    }
    let path = PathBuf::from(model_path);
    let xml = fs::read_to_string(&path)?;
    if vox.is_some() && [width, height, depth].into_iter().any(|side| side > 256) {
        return Err("VOX supports at most 256 cells per axis".into());
    }
    let mut model = Model::from_xml_3d(&xml, width, height, depth)?;
    let seed = seed.unwrap_or_else(rand::random);
    writeln!(io::stderr(), "seed: {seed}")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let completed = model.run(steps, &mut rng);
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(output, "steps: {completed}")?;
    if let Some(path) = vox {
        let bytes = model.grid().to_vox()?;
        fs::File::create_new(&path)?.write_all(&bytes)?;
        writeln!(output, "wrote {}", path.display())?;
    } else {
        for row in model.grid().rows() {
            writeln!(output, "{row}")?;
        }
    }
    Ok(())
}
