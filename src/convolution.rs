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
            "Moore" => Neighborhood::Moore,
            "VonNeumann" => Neighborhood::VonNeumann,
            value => return Err(error(format!("unknown 2D neighborhood '{value}'"))),
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
        for y in 0..grid.height {
            for x in 0..grid.width {
                let position = x + y * grid.width;
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        if dx == 0 && dy == 0
                            || matches!(neighborhood, Neighborhood::VonNeumann)
                                && dx != 0
                                && dy != 0
                        {
                            continue;
                        }
                        let mut next_x = x as isize + dx;
                        let mut next_y = y as isize + dy;
                        if periodic {
                            next_x = next_x.rem_euclid(grid.width as isize);
                            next_y = next_y.rem_euclid(grid.height as isize);
                        } else if next_x < 0
                            || next_y < 0
                            || next_x >= grid.width as isize
                            || next_y >= grid.height as isize
                        {
                            continue;
                        }
                        let next = next_x as usize + next_y as usize * grid.width;
                        counts[position][usize::from(grid.cells[next])] += 1;
                    }
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
