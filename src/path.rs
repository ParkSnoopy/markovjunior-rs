use std::collections::VecDeque;

use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use roxmltree::Node as XmlNode;

use crate::{
    error::{ModelError, error},
    grid::Grid,
};

#[derive(Debug)]
pub struct PathNode {
    starts: u16,
    finishes: u16,
    substrate: u16,
    color: u8,
    inertia: bool,
    longest: bool,
    edges: bool,
}

impl PathNode {
    pub fn load(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<Self, ModelError> {
        let from = xml
            .attribute("from")
            .ok_or_else(|| error("path node is missing 'from'"))?;
        let starts = wave(from, grid)?;
        let finishes = wave(
            xml.attribute("to")
                .ok_or_else(|| error("path node is missing 'to'"))?,
            grid,
        )?;
        let substrate = wave(
            xml.attribute("on")
                .ok_or_else(|| error("path node is missing 'on'"))?,
            grid,
        )?;
        let color_text = xml.attribute("color").unwrap_or(from);
        let mut color_chars = color_text.chars();
        let color_symbol = color_chars
            .next()
            .ok_or_else(|| error("path color must contain one symbol"))?;
        if color_chars.next().is_some() {
            return Err(error("path color must contain one symbol"));
        }
        let color = *grid
            .indices
            .get(&color_symbol)
            .ok_or_else(|| error("path color is not declared by grid values"))?;
        Ok(Self {
            starts,
            finishes,
            substrate,
            color,
            inertia: attribute_bool(xml, "inertia")?,
            longest: attribute_bool(xml, "longest")?,
            edges: attribute_bool(xml, "edges")?,
        })
    }

    pub fn apply<R: RngCore>(&self, grid: &mut Grid, rng: &mut R) -> bool {
        let mut distances = vec![None; grid.cells.len()];
        let mut frontier = VecDeque::new();
        let mut starts = Vec::new();
        for (position, cell) in grid.cells.iter().enumerate() {
            if self.starts & (1_u16 << cell) != 0 {
                starts.push(position);
            }
            if self.finishes & (1_u16 << cell) != 0 {
                distances[position] = Some(0);
                frontier.push_back(position);
            }
        }
        if starts.is_empty() || frontier.is_empty() {
            return false;
        }
        while let Some(position) = frontier.pop_front() {
            let Some(distance) = distances[position] else {
                return false;
            };
            let x = position % grid.width;
            let y = position / grid.width;
            for (dx, dy) in directions(x, y, grid.width, grid.height, self.edges) {
                let next_x = (x as isize + dx) as usize;
                let next_y = (y as isize + dy) as usize;
                let next = next_x + next_y * grid.width;
                let value = grid.cells[next];
                if distances[next].is_none()
                    && (self.substrate & (1_u16 << value) != 0
                        || self.starts & (1_u16 << value) != 0)
                {
                    distances[next] = Some(distance + 1);
                    if self.substrate & (1_u16 << value) != 0 {
                        frontier.push_back(next);
                    }
                }
            }
        }
        let mut local_rng = StdRng::seed_from_u64(rng.next_u64());
        let mut selected = None;
        let mut best = if self.longest {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        for position in starts {
            let Some(distance) = distances[position].filter(|distance| *distance > 0) else {
                continue;
            };
            let score = distance as f64 + local_rng.gen_range(0.0..0.1);
            if (self.longest && score > best) || (!self.longest && score < best) {
                best = score;
                selected = Some(position);
            }
        }
        let Some(mut position) = selected else {
            return false;
        };
        let mut previous = (0, 0);
        let search = PathSearch {
            width: grid.width,
            height: grid.height,
            distances: &distances,
            edges: self.edges,
            inertia: self.inertia,
        };
        loop {
            let Some(distance) = distances[position] else {
                return false;
            };
            let x = position % grid.width;
            let y = position / grid.width;
            let Some(direction) =
                search.choose_direction(position, distance, previous, &mut local_rng)
            else {
                return false;
            };
            let next_x = (x as isize + direction.0) as usize;
            let next_y = (y as isize + direction.1) as usize;
            position = next_x + next_y * grid.width;
            previous = direction;
            if distances[position] == Some(0) {
                return true;
            }
            grid.cells[position] = self.color;
        }
    }
}

fn wave(symbols: &str, grid: &Grid) -> Result<u16, ModelError> {
    if symbols.is_empty() {
        return Err(error("path symbol set cannot be empty"));
    }
    let mut result = 0;
    for symbol in symbols.chars() {
        let Some(index) = grid.indices.get(&symbol) else {
            return Err(error(format!(
                "path symbol '{symbol}' is not declared by grid values"
            )));
        };
        result |= 1_u16 << index;
    }
    Ok(result)
}

fn attribute_bool(xml: XmlNode<'_, '_>, name: &str) -> Result<bool, ModelError> {
    xml.attribute(name)
        .unwrap_or("false")
        .parse::<bool>()
        .map_err(|_| error(format!("path attribute '{name}' must be true or false")))
}

fn directions(x: usize, y: usize, width: usize, height: usize, edges: bool) -> Vec<(isize, isize)> {
    let mut result = Vec::with_capacity(if edges { 8 } else { 4 });
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 || (!edges && dx != 0 && dy != 0) {
                continue;
            }
            let next_x = x as isize + dx;
            let next_y = y as isize + dy;
            if next_x >= 0 && next_y >= 0 && next_x < width as isize && next_y < height as isize {
                result.push((dx, dy));
            }
        }
    }
    result
}

struct PathSearch<'a> {
    width: usize,
    height: usize,
    distances: &'a [Option<usize>],
    edges: bool,
    inertia: bool,
}

impl PathSearch<'_> {
    fn choose_direction<R: Rng>(
        &self,
        position: usize,
        distance: usize,
        previous: (isize, isize),
        rng: &mut R,
    ) -> Option<(isize, isize)> {
        let x = position % self.width;
        let y = position / self.width;
        if self.inertia && !self.edges && previous != (0, 0) {
            let next_x = x as isize + previous.0;
            let next_y = y as isize + previous.1;
            if next_x >= 0
                && next_y >= 0
                && next_x < self.width as isize
                && next_y < self.height as isize
            {
                let next = next_x as usize + next_y as usize * self.width;
                if self.distances[next] == Some(distance - 1) {
                    return Some(previous);
                }
            }
        }
        let candidates: Vec<_> = directions(x, y, self.width, self.height, self.edges)
            .into_iter()
            .filter(|(dx, dy)| {
                let next_x = (x as isize + dx) as usize;
                let next_y = (y as isize + dy) as usize;
                self.distances[next_x + next_y * self.width] == Some(distance - 1)
            })
            .collect();
        if self.inertia && previous != (0, 0) {
            let mut best = f64::NEG_INFINITY;
            let mut selected = None;
            for candidate in candidates {
                let dot = (candidate.0 * previous.0 + candidate.1 * previous.1) as f64;
                let candidate_length = candidate.0 * candidate.0 + candidate.1 * candidate.1;
                let previous_length = previous.0 * previous.0 + previous.1 * previous.1;
                let length_product = (candidate_length * previous_length) as f64;
                let score = dot / length_product.sqrt() + rng.gen_range(0.0..0.1);
                if score > best {
                    best = score;
                    selected = Some(candidate);
                }
            }
            selected
        } else if candidates.is_empty() {
            None
        } else {
            Some(candidates[rng.gen_range(0..candidates.len())])
        }
    }
}
