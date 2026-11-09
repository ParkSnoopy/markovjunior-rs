use rand::Rng;
use roxmltree::Node as XmlNode;

use crate::{
    error::{ModelError, error},
    grid::Grid,
    path::PathNode,
    rule::{Rule, apply_all, apply_one, resolve_symmetry},
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
    Path(PathNode),
}

impl Node {
    pub(crate) fn load(
        xml: XmlNode<'_, '_>,
        grid: &Grid,
        parent_symmetry: [bool; 8],
    ) -> Result<Self, ModelError> {
        match xml.tag_name().name() {
            "one" | "all" | "prl" => {
                let node_symmetry = resolve_symmetry(xml.attribute("symmetry"), parent_symmetry)?;
                let kind = match xml.tag_name().name() {
                    "one" => RuleKind::One,
                    "all" => RuleKind::All,
                    _ => RuleKind::Parallel,
                };
                let rule_nodes: Vec<_> = xml
                    .children()
                    .filter(|child| child.has_tag_name("rule"))
                    .collect();
                let rule_nodes = if rule_nodes.is_empty() {
                    vec![xml]
                } else {
                    rule_nodes
                };
                let mut rules = Vec::new();
                for rule_xml in rule_nodes {
                    let rule = Rule::load(rule_xml, grid)?;
                    let symmetry = resolve_symmetry(rule_xml.attribute("symmetry"), node_symmetry)?;
                    rules.extend(rule.symmetries(symmetry));
                }
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
                let symmetry = resolve_symmetry(xml.attribute("symmetry"), parent_symmetry)?;
                Ok(Self::Sequence {
                    children: load_children(xml, grid, symmetry)?,
                    next: 0,
                })
            }
            "markov" => {
                let symmetry = resolve_symmetry(xml.attribute("symmetry"), parent_symmetry)?;
                Ok(Self::Markov {
                    children: load_children(xml, grid, symmetry)?,
                })
            }
            "path" => Ok(Self::Path(PathNode::load(xml, grid)?)),
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
            Self::Path(path) => path.apply(grid, rng),
        }
    }
}

fn load_children(
    xml: XmlNode<'_, '_>,
    grid: &Grid,
    symmetry: [bool; 8],
) -> Result<Vec<Node>, ModelError> {
    xml.children()
        .filter(|child| {
            matches!(
                child.tag_name().name(),
                "one" | "all" | "prl" | "sequence" | "markov" | "path"
            )
        })
        .map(|child| Node::load(child, grid, symmetry))
        .collect()
}
