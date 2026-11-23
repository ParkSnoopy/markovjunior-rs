//! Checks the convolution node through the public model API.

use markovjunior_rs::Model;
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn cube_seed_changes_geometry_and_replays_it() -> Result<(), Box<dyn std::error::Error>> {
    let occupancy = |seed| -> Result<Vec<bool>, markovjunior_rs::ModelError> {
        let mut model = Model::from_xml_3d(include_str!("../models/FrameCube.xml"), 11, 11, 11)?;
        model.run(10, &mut StdRng::seed_from_u64(seed));
        Ok(model.grid().layers().flatten().map(|&v| v != 0).collect())
    };
    let original = occupancy(0)?;
    if original != occupancy(0)? || original == occupancy(1)? {
        return Err(
            "expected the same seed to replay braces and different seeds to move them".into(),
        );
    }
    Ok(())
}

#[test]
fn column_map_overlays_vertical_patterns_and_stays_finished()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = "<sequence values='AB' origin='true'><map values='.XY' scale='1 1 3'><rule in='*' out='X'/><rule in='B' out='Y . *'/></map></sequence>";
    let mut model = Model::from_xml(xml, 3, 1)?;
    let mut rng = StdRng::seed_from_u64(7);
    if model.run(1, &mut rng) != 1
        || model.run(10, &mut rng) != 0
        || model.run(10, &mut rng) != 0
        || model.grid().rows() != ["XXX", "...", ".Y."]
    {
        return Err("incorrect map transition or layer order".into());
    }
    Ok(())
}

#[test]
fn unsupported_map_geometry_does_not_silently_flatten() {
    for map in [
        "<map scale='2 1 2' values='.W'/>",
        "<map scale='1 1 2' values='.W'><rule in='A' out='W W W'/></map>",
        "<map scale='1 1 2' values='.W'/><one in='A' out='A'/>",
    ] {
        assert!(Model::from_xml(&format!("<sequence values='A'>{map}</sequence>"), 2, 2).is_err());
    }
}

#[test]
fn volume_exports_only_the_selected_neighbors() -> Result<(), Box<dyn std::error::Error>> {
    for (kernel, radius) in [("VonNeumann", 1), ("NoCorners", 2)] {
        let xml = format!(
            "<convolution values='BW' origin='true' neighborhood='{kernel}'><rule in='B' out='W' values='W' sum='1'/></convolution>"
        );
        let mut model = Model::from_xml_3d(&xml, 3, 5, 7)?;
        model.run(1, &mut StdRng::seed_from_u64(7));
        let bytes = model.grid().to_vox()?;
        let mut expected = Vec::new();
        for z in 2_u8..=4 {
            for y in 1_u8..=3 {
                for x in 0_u8..=2 {
                    if x.abs_diff(1) + y.abs_diff(2) + z.abs_diff(3) <= radius {
                        expected.extend([x, y, z, 1]);
                    }
                }
            }
        }
        if bytes.get(60..) != Some(expected.as_slice()) {
            return Err(format!("incorrect {kernel} voxel coordinates or palette index").into());
        }
    }
    Ok(())
}

#[test]
fn volume_rejects_unsupported_nodes_and_kernels() {
    for xml in [
        "<sequence values='BW'><one in='B' out='W'/></sequence>",
        "<convolution values='BW' neighborhood='Moore' in='B' out='W' sum='1' />",
    ] {
        assert!(Model::from_xml_3d(xml, 3, 3, 3).is_err());
    }
}

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
