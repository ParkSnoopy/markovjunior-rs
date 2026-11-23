//! Render the README's three rewrite examples from live model states.

use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use gif::{Encoder, Frame, Repeat};
use markovjunior_rs::Model;
use rand::{SeedableRng, rngs::StdRng};

const WIDTH: u16 = 41;
const HEIGHT: u16 = 33;
const SCALE: u16 = 8;

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "assets".into());
    std::fs::create_dir_all(&directory)?;
    for (name, xml, stride, symbols, colors) in [
        (
            "maze",
            include_str!("../models/MazeBacktracker.xml"),
            12,
            "BRGW",
            &[
                [20, 29, 39],
                [255, 137, 80],
                [54, 172, 155],
                [225, 239, 216],
            ][..],
        ),
        (
            "dungeon",
            include_str!("../models/BasicDungeonGrowth.xml"),
            1,
            "BRACDG",
            &[
                [22, 27, 35],
                [255, 142, 80],
                [120, 87, 78],
                [244, 177, 96],
                [249, 220, 166],
                [68, 154, 137],
            ][..],
        ),
        (
            "cave",
            include_str!("../models/Cave.xml"),
            1,
            "DA",
            &[[28, 39, 57], [160, 201, 216]][..],
        ),
    ] {
        let mut model = Model::from_xml(xml, WIDTH.into(), HEIGHT.into())?;
        let mut rng = StdRng::seed_from_u64(7);
        let mut states = vec![model.grid().rows()];
        let mut turns = 0;
        loop {
            let advanced = model.run(stride, &mut rng);
            turns += advanced;
            if advanced != 0 {
                states.push(model.grid().rows());
            }
            if advanced < stride {
                break;
            }
            if turns >= 10_000 {
                return Err(format!("{name} did not finish").into());
            }
        }
        let path = Path::new(&directory).join(format!("{name}.gif"));
        states.rotate_right(1); // Static previews show the finished map.
        write_animation(
            &path,
            &states,
            symbols,
            colors,
            if name == "cave" { 18 } else { 9 },
        )?;
        writeln!(
            std::io::stdout(),
            "{name}: {turns} turns, {} frames",
            states.len()
        )?;
    }
    Ok(())
}

fn write_animation(
    path: &Path,
    states: &[Vec<String>],
    symbols: &str,
    colors: &[[u8; 3]],
    delay: u16,
) -> Result<(), Box<dyn Error>> {
    let palette: Vec<_> = colors.iter().flatten().copied().collect();
    let mut encoder = Encoder::new(
        BufWriter::new(File::create(path)?),
        WIDTH * SCALE,
        HEIGHT * SCALE,
        &palette,
    )?;
    encoder.set_repeat(Repeat::Infinite)?;

    for (i, rows) in states.iter().enumerate() {
        let mut pixels = Vec::new();
        for row in rows {
            let mut line = Vec::new();
            for cell in row.chars() {
                let index = symbols.find(cell).ok_or("missing gallery color")? as u8;
                line.extend(std::iter::repeat_n(index, SCALE.into()));
            }
            for _ in 0..SCALE {
                pixels.extend_from_slice(&line);
            }
        }
        encoder.write_frame(&Frame {
            width: WIDTH * SCALE,
            height: HEIGHT * SCALE,
            delay: if i == 0 { 150 } else { delay },
            buffer: pixels.into(),
            ..Frame::default()
        })?;
    }
    encoder.into_inner()?.flush()?;
    Ok(())
}
