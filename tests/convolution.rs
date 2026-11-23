//! Checks the convolution node through the public model API.

use markovjunior_rs::Model;
use rand::{SeedableRng, rngs::StdRng};

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
