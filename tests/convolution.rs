//! Checks the convolution node through the public model API.

use markovjunior_rs::Model;
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn moore_neighborhood_changes_eight_cells() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<convolution values='BW' origin='true' neighborhood='Moore'><rule in='B' out='W' values='W' sum='1..1'/></convolution>";
    let mut model = Model::from_xml(xml, 5, 5)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(1, &mut rng);
    let whites = model.grid().rows().join("").matches('W').count();

    if turns != 1 || whites != 9 {
        return Err(std::io::Error::other(format!(
            "expected one Moore turn and 9 white cells, got {turns} turns and {whites} cells"
        ))
        .into());
    }
    Ok(())
}

#[test]
fn periodic_counts_include_wrapped_offsets_and_repeated_values()
-> Result<(), Box<dyn std::error::Error>> {
    for (periodic, expected) in [(false, "BWB"), (true, "WWW")] {
        let xml = format!(
            "<sequence values='BW' origin='true'><convolution neighborhood='Moore' periodic='{periodic}' in='B' out='W' values='WW' sum='5..6'/></sequence>"
        );
        let mut model = Model::from_xml(&xml, 3, 1)?;
        model.run(1, &mut StdRng::seed_from_u64(7));
        let row = model.grid().rows().join("");
        if row != expected {
            return Err(format!("periodic={periodic}: expected {expected}, got {row}").into());
        }
    }
    Ok(())
}

#[test]
fn von_neumann_neighborhood_changes_four_cells() -> Result<(), Box<dyn std::error::Error>> {
    let xml = "<convolution values='BW' origin='true' neighborhood='VonNeumann'><rule in='B' out='W' values='W' sum='1'/></convolution>";
    let mut model = Model::from_xml(xml, 5, 5)?;
    let mut rng = StdRng::seed_from_u64(7);

    let turns = model.run(1, &mut rng);
    let whites = model.grid().rows().join("").matches('W').count();

    if turns != 1 || whites != 5 {
        return Err(std::io::Error::other(format!(
            "expected one VonNeumann turn and 5 white cells, got {turns} turns and {whites} cells"
        ))
        .into());
    }
    Ok(())
}
