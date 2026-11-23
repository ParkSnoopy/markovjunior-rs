//! Export three voxel models and render sampled interpreter states.

mod common;

use std::{error::Error, fs, io::Write, path::Path};

use gif::{Encoder, Frame, Repeat};
use markovjunior_rs::{Grid, Model};
use rand::{SeedableRng, rngs::StdRng};

const WIDTH: u16 = 384;
const HEIGHT: u16 = 296;
const TOWER_VIEW: [u16; 2] = [240, 352];

fn main() -> Result<(), Box<dyn Error>> {
    let (directory, seed) = common::gallery_options()?;
    fs::create_dir_all(&directory)?;
    for (name, xml, [width, height, depth], colors) in [
        (
            "courtyard",
            include_str!("../models/Courtyard.xml"),
            [25, 25, 54],
            &[
                [135, 182, 206],
                [255, 187, 94],
                [109, 119, 148],
                [38, 99, 132],
                [99, 252, 242],
                [255, 75, 156],
                [248, 179, 101],
            ][..],
        ),
        (
            "frame-cube",
            include_str!("../models/FrameCube.xml"),
            [13, 13, 13],
            &[[255, 202, 119], [89, 190, 183]][..],
        ),
        (
            "terraced-island",
            include_str!("../models/TerracedIsland.xml"),
            [35, 29, 12],
            &[
                [164, 153, 137],
                [67, 155, 186],
                [138, 189, 107],
                [231, 203, 148],
                [58, 127, 94],
            ][..],
        ),
    ] {
        let mut model = Model::from_xml_3d(xml, width, height, depth)?;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut turns = 0;
        let view = if name == "courtyard" {
            TOWER_VIEW
        } else {
            [WIDTH, HEIGHT]
        };
        let mut frames = vec![volume_frame(model.grid(), view)?];
        let stride = if name == "frame-cube" { 6 } else { 2 };
        loop {
            let advanced = model.run(stride, &mut rng);
            if advanced != 0 {
                frames.push(volume_frame(model.grid(), view)?);
            }
            turns += advanced;
            if advanced < stride {
                break;
            }
            if turns >= 10_000 {
                return Err(format!("{name} did not finish").into());
            }
        }
        let base = Path::new(&directory).join(name);
        let grid = model.grid();
        fs::write(
            base.with_extension("vox"),
            grid.to_vox_with_palette(colors)?,
        )?;
        animation(&base.with_extension("gif"), colors, frames, view)?;
        if name == "courtyard" {
            building_samples(
                &directory.join("buildings-10.gif"),
                xml,
                [width, height, depth],
                colors,
                seed,
            )?;
        }
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

fn building_samples(
    path: &Path,
    xml: &str,
    [w, h, d]: [usize; 3],
    colors: &[[u8; 3]],
    seed: u64,
) -> Result<(), Box<dyn Error>> {
    let [tile_w, tile_h] = TOWER_VIEW;
    let width = tile_w * 5;
    let height = tile_h * 2;
    let mut sheet = vec![0; usize::from(width) * usize::from(height)];
    for i in 0..10 {
        let mut model = Model::from_xml_3d(xml, w, h, d)?;
        if model.run(
            1000,
            &mut StdRng::seed_from_u64(seed.wrapping_add(i as u64)),
        ) == 1000
        {
            return Err("building did not finish".into());
        }
        for (y, row) in volume_frame(model.grid(), TOWER_VIEW)?
            .chunks_exact(usize::from(tile_w))
            .enumerate()
        {
            let start = (i / 5 * usize::from(tile_h) + y) * usize::from(width)
                + i % 5 * usize::from(tile_w);
            sheet[start..start + row.len()].copy_from_slice(row);
        }
    }
    let mut encoder = Encoder::new(fs::File::create(path)?, width, height, &palette(colors))?;
    encoder.write_frame(&Frame {
        width,
        height,
        buffer: sheet.into(),
        ..Frame::default()
    })?;
    encoder.into_inner()?.flush()?;
    Ok(())
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
                pixels = volume_frame(grid, [WIDTH, HEIGHT])?;
            }
            frames.push(pixels);
        }
        animation(
            &directory.join(file),
            &[[120, 220, 193]],
            frames,
            [WIDTH, HEIGHT],
        )?;
    }
    Ok(())
}

fn background() -> Vec<u8> {
    vec![0; usize::from(WIDTH) * usize::from(HEIGHT)]
}

fn volume_frame(grid: &Grid, [w, h]: [u16; 2]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pixels = vec![0; usize::from(w) * usize::from(h)];
    let (width, height, depth) = (
        grid.width() as i32,
        grid.height() as i32,
        grid.layers().len() as i32,
    );
    let unit = ((i32::from(w) - 24) / (width + height))
        .min((i32::from(h) - 24) * 2 / (width + height + 2 * depth));
    if unit < 2 {
        return Err("volume too large for gallery preview".into());
    }
    let x0 = (i32::from(w) - (width - height) * unit) / 2;
    let y0 = i32::midpoint(i32::from(h) - (width + height) * unit / 2, depth * unit);

    for (z, layer) in grid.layers().enumerate() {
        for (i, &value) in layer.iter().enumerate().filter(|&(_, v)| *v != 0) {
            let (x, y) = ((i % grid.width()) as i32, (i / grid.width()) as i32);
            let cx = x0 + (x - y) * unit;
            let cy = y0 + (x + y) * unit / 2 - z as i32 * unit;
            let top = 1 + (value - 1) * 3;
            for dx in -unit..unit {
                let half = (2 * unit - (2 * dx + 1).abs()) / 4;
                for dy in -half..unit + half {
                    let index = (cx + dx + (cy + dy) * i32::from(w)) as usize;
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
    }
    Ok(pixels)
}

fn animation(
    path: &Path,
    colors: &[[u8; 3]],
    frames: Vec<Vec<u8>>,
    [width, height]: [u16; 2],
) -> Result<(), Box<dyn Error>> {
    let palette = palette(colors);
    let last = frames.len().checked_sub(1).ok_or("no frames")?;
    let mut encoder = Encoder::new(fs::File::create(path)?, width, height, &palette)?;
    encoder.set_repeat(Repeat::Infinite)?;
    for (i, buffer) in frames.into_iter().enumerate() {
        encoder.write_frame(&Frame {
            width,
            height,
            delay: if i == last { 160 } else { 18 },
            buffer: buffer.into(),
            ..Frame::default()
        })?;
    }
    encoder.into_inner()?.flush()?;
    Ok(())
}

fn palette(colors: &[[u8; 3]]) -> Vec<u8> {
    let mut palette = vec![22, 31, 40];
    for color in colors {
        for light in [100_u16, 79, 62] {
            palette.extend(color.map(|c| (u16::from(c) * light / 100) as u8));
        }
    }
    palette
}
