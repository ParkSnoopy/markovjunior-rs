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

#[test]
fn branch_symmetry_limits_child_rule_orientations() -> Result<(), Box<dyn std::error::Error>> {
    let xml =
        "<sequence values='BW' origin='true' symmetry='()'><one in='WB' out='WW'/></sequence>";
    let mut model = Model::from_xml(xml, 5, 5)?;
    let mut rng = StdRng::seed_from_u64(7);

    model.run(1, &mut rng);
    let rows = model.grid().rows();

    if rows[2] != "BBWWB" {
        return Err(std::io::Error::other(format!(
            "identity symmetry should grow right only, got {}",
            rows[2]
        ))
        .into());
    }
    Ok(())
}

#[test]
fn rule_symmetry_overrides_inherited_symmetry() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<sequence values='BW' origin='true' symmetry='()'><one><rule in='WB' out='WW' symmetry='(x)'/></one></sequence>";
    let mut model = Model::from_xml(xml, 5, 5)?;
    let mut rng = StdRng::seed_from_u64(7);

    model.run(1, &mut rng);
    let rows = model.grid().rows();

    if rows[2] != "BBWWB" && rows[2] != "BWWBB" {
        return Err(std::io::Error::other(format!(
            "horizontal reflection should grow horizontally, got {}",
            rows[2]
        ))
        .into());
    }
    if rows
        .iter()
        .enumerate()
        .any(|(y, row)| y != 2 && row != "BBBBB")
    {
        return Err(std::io::Error::other("horizontal symmetry changed another row").into());
    }
    Ok(())
}

#[test]
fn model_rejects_unknown_symmetry_groups() {
    let result = Model::from_xml("<one values='BW' in='WB' out='WW' symmetry='(q)'/>", 5, 5);

    assert!(result.is_err());
}

#[test]
fn union_input_matches_each_member_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<sequence values='BWG'><union symbol='X' values='BW'/><one in='X' out='G' steps='1'/></sequence>";
    let mut model = Model::from_xml(xml, 4, 4)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(10, &mut rng);
    let greens = model.grid().rows().join("").matches('G').count();

    if turns != 1 || greens != 1 {
        return Err(std::io::Error::other(format!(
            "expected one union rewrite, got {turns} turns and {greens} green cells"
        ))
        .into());
    }
    Ok(())
}

#[test]
fn model_rejects_union_members_outside_grid_values() {
    let result = Model::from_xml(
        "<sequence values='BW'><union symbol='X' values='Q'/><one in='X' out='B'/></sequence>",
        4,
        4,
    );

    assert!(result.is_err());
}
