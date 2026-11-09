use rand::Rng;
use roxmltree::Node as XmlNode;

use crate::{
    error::{ModelError, error},
    grid::Grid,
};

#[derive(Debug, Clone)]
pub struct Rule {
    input: Vec<u16>,
    output: Vec<Option<char>>,
    width: usize,
    height: usize,
    probability: f64,
}

impl Rule {
    pub fn load(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<Self, ModelError> {
        let input = xml
            .attribute("in")
            .ok_or_else(|| error("rule is missing 'in'"))?;
        let output = xml
            .attribute("out")
            .ok_or_else(|| error("rule is missing 'out'"))?;
        let input_rows = parse_pattern(input)?;
        let output_rows = parse_pattern(output)?;
        if input_rows.len() != output_rows.len() || input_rows[0].len() != output_rows[0].len() {
            return Err(error(
                "input and output patterns must have the same dimensions",
            ));
        }
        if input_rows
            .iter()
            .flatten()
            .any(|symbol| !grid.waves.contains_key(symbol))
            || output_rows
                .iter()
                .flatten()
                .any(|symbol| *symbol != '*' && !grid.indices.contains_key(symbol))
        {
            return Err(error("rule contains a symbol not declared by grid values"));
        }
        let width = input_rows[0].len();
        let height = input_rows.len();
        let input = input_rows
            .into_iter()
            .flatten()
            .map(|symbol| grid.waves[&symbol])
            .collect();
        let output = output_rows
            .into_iter()
            .flatten()
            .map(|symbol| (symbol != '*').then_some(symbol))
            .collect();
        let probability = xml
            .attribute("p")
            .unwrap_or("1")
            .parse::<f64>()
            .map_err(|_| error("rule probability must be a number"))?;
        if !(0.0..=1.0).contains(&probability) {
            return Err(error("rule probability must be between 0 and 1"));
        }
        Ok(Self {
            input,
            output,
            width,
            height,
            probability,
        })
    }

    pub fn symmetries(&self, subgroup: [bool; 8]) -> Vec<Self> {
        let mut result = Vec::with_capacity(8);
        let mut rotated = self.clone();
        for rotation in 0..4 {
            if subgroup[rotation * 2] {
                push_unique(&mut result, rotated.clone());
            }
            if subgroup[rotation * 2 + 1] {
                push_unique(&mut result, rotated.reflected());
            }
            rotated = rotated.rotate();
        }
        result
    }

    fn rotate(&self) -> Self {
        let mut input = vec![0; self.input.len()];
        let mut output = vec![None; self.output.len()];
        for y in 0..self.width {
            for x in 0..self.height {
                let source = self.width - 1 - y + x * self.width;
                let target = x + y * self.height;
                input[target] = self.input[source];
                output[target] = self.output[source];
            }
        }
        Self {
            input,
            output,
            width: self.height,
            height: self.width,
            probability: self.probability,
        }
    }

    fn reflected(&self) -> Self {
        let mut reflected = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let source = self.width - 1 - x + y * self.width;
                let target = x + y * self.width;
                reflected.input[target] = self.input[source];
                reflected.output[target] = self.output[source];
            }
        }
        reflected
    }

    fn matches(&self, grid: &Grid, x: usize, y: usize) -> bool {
        (0..self.height).all(|dy| {
            (0..self.width).all(|dx| {
                let value = grid.cells[x + dx + (y + dy) * grid.width];
                self.input[dx + dy * self.width] & (1_u16 << value) != 0
            })
        })
    }

    fn apply(&self, grid: &mut Grid, x: usize, y: usize) {
        for dy in 0..self.height {
            for dx in 0..self.width {
                if let Some(symbol) = self.output[dx + dy * self.width] {
                    grid.cells[x + dx + (y + dy) * grid.width] = grid.indices[&symbol];
                }
            }
        }
    }
}

pub fn resolve_symmetry(value: Option<&str>, parent: [bool; 8]) -> Result<[bool; 8], ModelError> {
    let Some(value) = value else {
        return Ok(parent);
    };
    match value {
        "()" => Ok([true, false, false, false, false, false, false, false]),
        "(x)" => Ok([true, true, false, false, false, false, false, false]),
        "(y)" => Ok([true, false, false, false, false, true, false, false]),
        "(x)(y)" => Ok([true, true, false, false, true, true, false, false]),
        "(xy+)" => Ok([true, false, true, false, true, false, true, false]),
        "(xy)" => Ok([true; 8]),
        _ => Err(error(format!("unknown square symmetry '{value}'"))),
    }
}

fn push_unique(rules: &mut Vec<Rule>, candidate: Rule) {
    let is_duplicate = rules.iter().any(|rule| {
        rule.width == candidate.width
            && rule.height == candidate.height
            && rule.input == candidate.input
            && rule.output == candidate.output
    });
    if !is_duplicate {
        rules.push(candidate);
    }
}

fn parse_pattern(source: &str) -> Result<Vec<Vec<char>>, ModelError> {
    let rows: Vec<Vec<char>> = source
        .split('/')
        .map(|row| {
            row.chars()
                .filter(|symbol| !symbol.is_whitespace())
                .collect()
        })
        .collect();
    if rows.is_empty() || rows[0].is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
        return Err(error("rule pattern must be a non-empty rectangle"));
    }
    Ok(rows)
}

pub fn apply_one<R: Rng>(rules: &[Rule], grid: &mut Grid, rng: &mut R) -> bool {
    for rule in rules {
        let mut matches = Vec::new();
        if rule.width > grid.width || rule.height > grid.height {
            continue;
        }
        for y in 0..=grid.height - rule.height {
            for x in 0..=grid.width - rule.width {
                if rule.matches(grid, x, y) {
                    matches.push((x, y));
                }
            }
        }
        if !matches.is_empty() {
            let (x, y) = matches[rng.gen_range(0..matches.len())];
            rule.apply(grid, x, y);
            return true;
        }
    }
    false
}

pub fn apply_all<R: Rng>(rules: &[Rule], grid: &mut Grid, rng: &mut R, parallel: bool) -> bool {
    let mut candidates = Vec::new();
    for rule in rules {
        if rule.width > grid.width || rule.height > grid.height {
            continue;
        }
        for y in 0..=grid.height - rule.height {
            for x in 0..=grid.width - rule.width {
                if rule.matches(grid, x, y) {
                    candidates.push((rule, x, y));
                }
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }
    for index in (1..candidates.len()).rev() {
        candidates.swap(index, rng.gen_range(0..=index));
    }
    let mut reserved = vec![false; grid.cells.len()];
    let mut writes = Vec::new();
    let mut applied = false;
    for (rule, x, y) in candidates {
        if parallel && rng.gen_range(0.0..1.0) > rule.probability {
            continue;
        }
        let grid_width = grid.width;
        let positions: Vec<usize> = (0..rule.height)
            .flat_map(|dy| (0..rule.width).map(move |dx| x + dx + (y + dy) * grid_width))
            .collect();
        let output_positions: Vec<usize> = positions
            .iter()
            .zip(&rule.output)
            .filter_map(|(position, output)| output.map(|_| *position))
            .collect();
        if !parallel && output_positions.iter().any(|position| reserved[*position]) {
            continue;
        }
        applied = true;
        for (position, output) in positions.into_iter().zip(&rule.output) {
            if let Some(symbol) = output {
                let value = grid.indices[symbol];
                writes.push((position, value));
                reserved[position] = true;
            }
        }
    }
    for (position, value) in writes {
        grid.cells[position] = value;
    }
    applied
}
