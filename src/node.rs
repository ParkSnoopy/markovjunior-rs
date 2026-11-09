use rand::Rng;
use roxmltree::Node as XmlNode;

use crate::{
    error::{ModelError, error},
    grid::Grid,
    rule::{Rule, apply_all, apply_one},
};

#[derive(Debug, Clone, Copy)]
pub enum RuleKind {
    One,
    All,
    Parallel,
}

#[derive(Debug)]
pub enum Node {
    Rules {
        kind: RuleKind,
        rules: Vec<Rule>,
        steps: usize,
        count: usize,
    },
    Sequence {
        children: Vec<Self>,
        next: usize,
    },
    Markov {
        children: Vec<Self>,
    },
}

impl Node {
    pub(crate) fn load(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<Self, ModelError> {
        match xml.tag_name().name() {
            "one" | "all" | "prl" => {
                let kind = match xml.tag_name().name() {
                    "one" => RuleKind::One,
                    "all" => RuleKind::All,
                    _ => RuleKind::Parallel,
                };
                let rule_nodes: Vec<_> = xml
                    .children()
                    .filter(|child| child.has_tag_name("rule"))
                    .collect();
                let base_rules = if rule_nodes.is_empty() {
                    vec![Rule::load(xml, grid)?]
                } else {
                    rule_nodes
                        .into_iter()
                        .map(|rule| Rule::load(rule, grid))
                        .collect::<Result<Vec<_>, _>>()?
                };
                let rules = base_rules.iter().flat_map(Rule::symmetries).collect();
                let steps = xml
                    .attribute("steps")
                    .unwrap_or("0")
                    .parse()
                    .map_err(|_| error("steps must be a non-negative integer"))?;
                Ok(Self::Rules {
                    kind,
                    rules,
                    steps,
                    count: 0,
                })
            }
            "sequence" => {
                Ok(Self::Sequence {
                    children: load_children(xml, grid)?,
                    next: 0,
                })
            }
            "markov" => {
                Ok(Self::Markov {
                    children: load_children(xml, grid)?,
                })
            }
            kind => Err(error(format!("unsupported node type '{kind}'"))),
        }
    }

    pub(crate) fn tick<R: Rng>(&mut self, grid: &mut Grid, rng: &mut R) -> bool {
        match self {
            Self::Rules {
                kind,
                rules,
                steps,
                count,
            } => {
                if *steps > 0 && *count >= *steps {
                    *count = 0;
                    return false;
                }
                let applied = match kind {
                    RuleKind::One => apply_one(rules, grid, rng),
                    RuleKind::All => apply_all(rules, grid, rng, false),
                    RuleKind::Parallel => apply_all(rules, grid, rng, true),
                };
                if applied {
                    *count += 1;
                } else {
                    *count = 0;
                }
                applied
            }
            Self::Sequence { children, next } => {
                while *next < children.len() {
                    if children[*next].tick(grid, rng) {
                        return true;
                    }
                    *next += 1;
                }
                *next = 0;
                false
            }
            Self::Markov { children } => children.iter_mut().any(|child| child.tick(grid, rng)),
        }
    }
}

fn load_children(xml: XmlNode<'_, '_>, grid: &Grid) -> Result<Vec<Node>, ModelError> {
    xml.children()
        .filter(|child| {
            matches!(
                child.tag_name().name(),
                "one" | "all" | "prl" | "sequence" | "markov"
            )
        })
        .map(|child| Node::load(child, grid))
        .collect()
}
