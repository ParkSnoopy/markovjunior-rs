//! Checks model loading and rewrite behavior through the public API.

use markovjunior_rs::Model;
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn basic_rule_changes_one_cell_per_turn() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<one values='BW' origin='true' in='B' out='W'/>";
    let mut model = Model::from_xml(xml, 4, 4)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(1, &mut rng);
    let whites = model.grid().rows().join("").matches('W').count();

    if turns != 1 {
        return Err(std::io::Error::other(format!("expected 1 turn, got {turns}")).into());
    }
    if whites != 2 {
        return Err(std::io::Error::other(format!("expected 2 white cells, got {whites}")).into());
    }
    Ok(())
}

#[test]
fn growth_rule_expands_from_its_origin() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<one values='BW' origin='true' in='WB' out='WW'/>";
    let mut model = Model::from_xml(xml, 5, 5)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(20, &mut rng);
    let whites = model.grid().rows().join("").matches('W').count();

    if turns != 20 || whites != 21 {
        return Err(std::io::Error::other(format!(
            "expected 20 growth turns and 21 white cells, got {turns} turns and {whites} white cells"
        ))
        .into());
    }
    Ok(())
}

#[test]
fn model_rejects_rules_with_undeclared_symbols() {
    let result = Model::from_xml("<one values='BW' in='B' out='X'/>", 4, 4);

    assert!(result.is_err());
}

#[test]
fn path_node_connects_a_start_to_a_reachable_goal() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<sequence values='BRWG' origin='true'><one in='B' out='W' steps='1'/><path from='R' to='W' on='B' color='G' inertia='true'/></sequence>";
    let mut model = Model::from_xml(xml, 9, 9)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(10, &mut rng);
    let green = model.grid().rows().join("").matches('G').count();

    if turns < 2 || green == 0 {
        return Err(std::io::Error::other(format!(
            "expected a path, got {turns} turns and {green} path cells"
        ))
        .into());
    }
    Ok(())
}
