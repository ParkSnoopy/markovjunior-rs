use rand::Rng;
use roxmltree::Node as XmlNode;

use crate::{
    error::{ModelError, error},
    grid::Grid,
};

#[derive(Debug)]
pub struct ConvolutionNode {
    rules: Vec<ConvolutionRule>,
    neighborhood: Neighborhood,
    periodic: bool,
    steps: Option<usize>,
    count: usize,
}

#[derive(Debug, Clone, Copy)]
enum Neighborhood {
    Moore,
    NoCorners,
    VonNeumann,
}

#[derive(Debug)]
struct ConvolutionRule {
    input: u8,
    output: u8,
    values: Vec<u8>,
    sums: Option<u32>,
    probability: f64,
}

impl ConvolutionNode {
    pub fn load(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<Self, ModelError> {
        let neighborhood = match xml
            .attribute("neighborhood")
            .ok_or_else(|| error("convolution node is missing 'neighborhood'"))?
        {
            "Moore" if grid.depth == 1 => Neighborhood::Moore,
            "NoCorners" if grid.depth > 1 => Neighborhood::NoCorners,
            "VonNeumann" => Neighborhood::VonNeumann,
            value => {
                return Err(error(format!(
                    "unsupported neighborhood '{value}' for depth {}",
                    grid.depth
                )));
            }
        };
        let steps = xml
            .attribute("steps")
            .unwrap_or("-1")
            .parse::<isize>()
            .map_err(|_| error("convolution steps must be an integer"))?;
        let rules: Vec<_> = xml
            .children()
            .filter(|child| child.has_tag_name("rule"))
            .collect();
        let rules = if rules.is_empty() {
            vec![load_rule(xml, grid)?]
        } else {
            rules
                .into_iter()
                .map(|rule| load_rule(rule, grid))
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Self {
            rules,
            neighborhood,
            periodic: parse_bool(xml.attribute("periodic"), "periodic")?,
            steps: (steps > 0).then_some(steps as usize),
            count: 0,
        })
    }

    pub fn tick<R: Rng>(&mut self, grid: &mut Grid, rng: &mut R) -> bool {
        if self.steps.is_some_and(|steps| self.count >= steps) {
            return false;
        }
        let sums = Self::neighbor_counts(grid, self.neighborhood, self.periodic);
        let mut changed = false;
        for (position, cell) in grid.cells.iter_mut().enumerate() {
            let input = *cell;
            for rule in &self.rules {
                if input != rule.input || *cell == rule.output {
                    continue;
                }
                if rule.probability < 1.0 && rng.gen_range(0.0..1.0) >= rule.probability {
                    continue;
                }
                if rule.sums.is_some_and(|allowed| {
                    let sum: usize = rule
                        .values
                        .iter()
                        .map(|&symbol| usize::from(sums[position][usize::from(symbol)]))
                        .sum();
                    sum >= u32::BITS as usize || allowed & (1_u32 << sum) == 0
                }) {
                    continue;
                }
                *cell = rule.output;
                changed = true;
                break;
            }
        }
        self.count += 1;
        changed
    }

    fn neighbor_counts(grid: &Grid, neighborhood: Neighborhood, periodic: bool) -> Vec<[u8; 8]> {
        let mut counts = vec![[0; 8]; grid.cells.len()];
        let radius_z = isize::from(u8::from(grid.depth > 1));
        let max_distance = match neighborhood {
            Neighborhood::VonNeumann => 1,
            Neighborhood::Moore | Neighborhood::NoCorners => 2,
        };
        let mut offsets = Vec::new();
        for dz in -radius_z..=radius_z {
            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    if (1..=max_distance).contains(&(dx.abs() + dy.abs() + dz.abs())) {
                        offsets.push((dx, dy, dz));
                    }
                }
            }
        }
        let axis = |coord: usize, delta: isize, size: usize| {
            let n = coord as isize + delta;
            if periodic {
                Some(n.rem_euclid(size as isize) as usize)
            } else {
                (0..size as isize).contains(&n).then_some(n as usize)
            }
        };
        for (i, count) in counts.iter_mut().enumerate() {
            let (x, row) = (i % grid.width, i / grid.width);
            let (y, z) = (row % grid.height, row / grid.height);
            for &(dx, dy, dz) in &offsets {
                if let (Some(nx), Some(ny), Some(nz)) = (
                    axis(x, dx, grid.width),
                    axis(y, dy, grid.height),
                    axis(z, dz, grid.depth),
                ) {
                    let value = grid.cells[nx + grid.width * (ny + grid.height * nz)];
                    count[usize::from(value)] += 1;
                }
            }
        }
        counts
    }
}

fn load_rule(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<ConvolutionRule, ModelError> {
    let input = single_symbol(xml.attribute("in"), "in", grid)?;
    let output = single_symbol(xml.attribute("out"), "out", grid)?;
    let values = xml.attribute("values");
    let sums = xml.attribute("sum");
    if values.is_some() != sums.is_some() {
        return Err(error("convolution rules require both 'values' and 'sum'"));
    }
    let mut counted_values = Vec::new();
    if let Some(values) = values {
        for symbol in values.chars() {
            let Some(index) = grid.indices.get(&symbol) else {
                return Err(error(format!(
                    "convolution value '{symbol}' is not declared by grid values"
                )));
            };
            counted_values.push(*index);
        }
    }
    let sums = sums.map(parse_sums).transpose()?;
    let probability = xml
        .attribute("p")
        .unwrap_or("1")
        .parse::<f64>()
        .map_err(|_| error("convolution probability must be a number"))?;
    if !(0.0..=1.0).contains(&probability) {
        return Err(error("convolution probability must be between 0 and 1"));
    }
    Ok(ConvolutionRule {
        input,
        output,
        values: counted_values,
        sums,
        probability,
    })
}

fn single_symbol(value: Option<&str>, name: &str, grid: &Grid) -> Result<u8, ModelError> {
    let value = value.ok_or_else(|| error(format!("convolution rule is missing '{name}'")))?;
    let mut symbols = value.chars();
    let symbol = symbols
        .next()
        .ok_or_else(|| error(format!("convolution '{name}' must be one grid symbol")))?;
    if symbols.next().is_some() {
        return Err(error(format!(
            "convolution '{name}' must be one grid symbol"
        )));
    }
    grid.indices
        .get(&symbol)
        .copied()
        .ok_or_else(|| error(format!("convolution symbol '{symbol}' is not declared")))
}

fn parse_sums(source: &str) -> Result<u32, ModelError> {
    let mut allowed = 0;
    for interval in source.split(',') {
        let interval = interval.trim();
        let (start, end) = interval
            .split_once("..")
            .map_or_else(
                || interval.parse::<u32>().map(|sum| (sum, sum)),
                |(start, end)| {
                    start
                        .trim()
                        .parse::<u32>()
                        .and_then(|start| end.trim().parse::<u32>().map(|end| (start, end)))
                },
            )
            .map_err(|_| error("convolution sum must contain integers or ranges"))?;
        if start > end || end > 27 {
            return Err(error(
                "convolution sum range must be ordered and at most 27",
            ));
        }
        for sum in start..=end {
            allowed |= 1_u32 << sum;
        }
    }
    Ok(allowed)
}

fn parse_bool(value: Option<&str>, name: &str) -> Result<bool, ModelError> {
    match value.unwrap_or("false") {
        value if value.eq_ignore_ascii_case("true") => Ok(true),
        value if value.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(error(format!("convolution '{name}' must be true or false"))),
    }
}
