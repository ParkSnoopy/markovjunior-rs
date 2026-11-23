//! Render the README's rewrite examples from live model states.

mod common;

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
    let (directory, seed) = common::gallery_options()?;
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
        (
            "river",
            include_str!("../models/River.xml"),
            32,
            "BWRUGE",
            &[
                [32, 43, 57],
                [224, 211, 162],
                [179, 126, 98],
                [79, 168, 218],
                [164, 201, 115],
                [56, 116, 84],
            ][..],
        ),
        (
            "flowers",
            include_str!("../models/Flowers.xml"),
            2,
            "NGUEPY",
            &[
                [67, 48, 52],
                [117, 167, 90],
                [39, 58, 88],
                [79, 183, 130],
                [239, 139, 165],
                [255, 208, 112],
            ][..],
        ),
        (
            "rooms-and-mazes",
            include_str!("../models/NystromDungeon.xml"),
            8,
            "BPWRG",
            &[
                [26, 34, 44],
                [93, 107, 125],
                [236, 210, 169],
                [240, 135, 78],
                [79, 166, 146],
            ][..],
        ),
    ] {
        render(&directory, name, xml, stride, symbols, colors, seed)?;
    }
    Ok(())
}

fn render(
    directory: &Path,
    name: &str,
    xml: &str,
    stride: usize,
    symbols: &str,
    colors: &[[u8; 3]],
    seed: u64,
) -> Result<(), Box<dyn Error>> {
    let mut model = Model::from_xml(xml, WIDTH.into(), HEIGHT.into())?;
    let mut rng = StdRng::seed_from_u64(seed);
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
    let path = directory.join(format!("{name}.gif"));
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
