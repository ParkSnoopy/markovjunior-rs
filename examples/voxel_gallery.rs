//! Export three voxel models and illustrate their geometry one z layer at a time.

mod common;

use std::{error::Error, fs, io::Write, path::Path};

use gif::{Encoder, Frame, Repeat};
use markovjunior_rs::{Grid, Model};
use rand::{SeedableRng, rngs::StdRng};

const WIDTH: u16 = 384;
const HEIGHT: u16 = 296;

fn main() -> Result<(), Box<dyn Error>> {
    let (directory, seed) = common::gallery_options()?;
    fs::create_dir_all(&directory)?;
    for (name, xml, [width, height, depth], colors) in [
        (
            "courtyard",
            include_str!("../models/Courtyard.xml"),
            [23, 17, 1],
            &[
                [158, 155, 146],
                [239, 220, 187],
                [71, 145, 165],
                [107, 82, 57],
                [109, 158, 106],
            ][..],
        ),
        (
            "frame-cube",
            include_str!("../models/FrameCube.xml"),
            [11, 11, 11],
            &[[89, 190, 183], [255, 202, 119], [100, 151, 186]][..],
        ),
        (
            "terraced-island",
            include_str!("../models/TerracedIsland.xml"),
            [35, 29, 1],
            &[
                [67, 155, 186],
                [231, 203, 148],
                [164, 153, 137],
                [138, 189, 107],
                [58, 127, 94],
                [110, 79, 56],
            ][..],
        ),
    ] {
        let mut model = Model::from_xml_3d(xml, width, height, depth)?;
        let turns = model.run(10_000, &mut StdRng::seed_from_u64(seed));
        if turns == 10_000 {
            return Err(format!("{name} did not finish").into());
        }
        let base = Path::new(&directory).join(name);
        let grid = model.grid();
        fs::write(
            base.with_extension("vox"),
            grid.to_vox_with_palette(colors)?,
        )?;
        preview(&base.with_extension("gif"), grid, colors)?;
        writeln!(
            std::io::stdout(),
            "{name}: {} × {} × {}, {turns} turns",
            grid.width(),
            grid.height(),
            grid.layers().len()
        )?;
    }
    growth_pair(&directory, seed)
}

fn growth_pair(directory: &Path, seed: u64) -> Result<(), Box<dyn Error>> {
    const SIDE: usize = 31;
    for (depth, file) in [(1, "growth-2d.gif"), (SIDE, "growth-3d.gif")] {
        let mut model =
            Model::from_xml_3d(include_str!("../models/VoxelGrowth.xml"), SIDE, SIDE, depth)?;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut frames = Vec::new();
        for turn in 0..=14 {
            if turn != 0 && model.run(1, &mut rng) == 0 {
                break;
            }
            let grid = model.grid();
            let mut pixels = background();
            if depth == 1 {
                let scale = usize::from(HEIGHT) / SIDE;
                for (layer_index, &cell) in grid
                    .layers()
                    .flatten()
                    .enumerate()
                    .filter(|&(_, v)| *v != 0)
                {
                    let x = (usize::from(WIDTH) - SIDE * scale) / 2 + (layer_index % SIDE) * scale;
                    let y = (usize::from(HEIGHT) - SIDE * scale) / 2 + (layer_index / SIDE) * scale;
                    for row in y..y + scale - 1 {
                        let start = row * usize::from(WIDTH) + x;
                        pixels[start..start + scale - 1].fill(cell);
                    }
                }
            } else {
                draw_volume(grid, &mut pixels, |_| {})?;
            }
            frames.push(pixels);
        }
        animation(&directory.join(file), &[[120, 220, 193]], frames)?;
    }
    Ok(())
}

fn background() -> Vec<u8> {
    vec![0; usize::from(WIDTH) * usize::from(HEIGHT)]
}

fn preview(path: &Path, grid: &Grid, colors: &[[u8; 3]]) -> Result<(), Box<dyn Error>> {
    let mut frames = Vec::new();
    draw_volume(grid, &mut background(), |pixels| {
        frames.push(pixels.to_vec());
    })?;
    animation(path, colors, frames)
}

fn draw_volume(
    grid: &Grid,
    pixels: &mut [u8],
    mut layer_done: impl FnMut(&[u8]),
) -> Result<(), Box<dyn Error>> {
    let (width, height, depth) = (
        grid.width() as i32,
        grid.height() as i32,
        grid.layers().len() as i32,
    );
    let unit = ((i32::from(WIDTH) - 24) / (width + height))
        .min((i32::from(HEIGHT) - 24) * 2 / (width + height + 2 * depth));
    if unit < 2 {
        return Err("volume too large for gallery preview".into());
    }
    let x0 = (i32::from(WIDTH) - (width - height) * unit) / 2;
    let y0 = i32::midpoint(
        i32::from(HEIGHT) - (width + height) * unit / 2,
        depth * unit,
    );

    for (z, layer) in grid.layers().enumerate() {
        for (i, &value) in layer.iter().enumerate().filter(|&(_, v)| *v != 0) {
            let (x, y) = ((i % grid.width()) as i32, (i / grid.width()) as i32);
            let cx = x0 + (x - y) * unit;
            let cy = y0 + (x + y) * unit / 2 - z as i32 * unit;
            let top = 1 + (value - 1) * 3;
            for dx in -unit..unit {
                let half = (2 * unit - (2 * dx + 1).abs()) / 4;
                for dy in -half..unit + half {
                    let index = (cx + dx + (cy + dy) * i32::from(WIDTH)) as usize;
                    pixels[index] = if dy < half {
                        top
                    } else if dx < 0 {
                        top + 1
                    } else {
                        top + 2
                    };
                }
            }
        }
        if layer.iter().any(|&v| v != 0) {
            layer_done(pixels);
        }
    }
    Ok(())
}

fn animation(
    path: &Path,
    colors: &[[u8; 3]],
    mut frames: Vec<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let mut palette = vec![22, 31, 40];
    for color in colors {
        for light in [100_u16, 79, 62] {
            palette.extend(color.map(|c| (u16::from(c) * light / 100) as u8));
        }
    }
    if frames.is_empty() {
        return Err("gallery volume is empty".into());
    }
    frames.rotate_right(1);
    let mut encoder = Encoder::new(fs::File::create(path)?, WIDTH, HEIGHT, &palette)?;
    encoder.set_repeat(Repeat::Infinite)?;
    for (i, buffer) in frames.into_iter().enumerate() {
        encoder.write_frame(&Frame {
            width: WIDTH,
            height: HEIGHT,
            delay: if i == 0 { 160 } else { 18 },
            buffer: buffer.into(),
            ..Frame::default()
        })?;
    }
    encoder.into_inner()?.flush()?;
    Ok(())
}
