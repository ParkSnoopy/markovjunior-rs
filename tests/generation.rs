//! Check example geometry before rendering or assigning colors.

use std::collections::HashSet;

use markovjunior_rs::{Model, ModelError};
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn river_grows_two_fronts_then_marks_only_their_contact() -> Result<(), ModelError> {
    const SIDE: usize = 13;
    for seed in 0..10 {
        let model = Model::from_xml(include_str!("../models/River.xml"), SIDE, SIDE)?;
        check_fronts(model, seed);
    }
    Ok(())
}

fn check_fronts(mut model: Model, seed: u64) {
    let side = model.grid().width();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut before = model.grid().rows().concat().into_bytes();
    assert!(before.iter().all(|&v| v == b'B'));
    for turn in 0..side * side {
        assert_eq!(model.run(1, &mut rng), 1);
        let after = model.grid().rows().concat().into_bytes();
        let changes: Vec<_> = (0..after.len())
            .filter(|&i| before[i] != after[i])
            .collect();
        assert_eq!(changes.len(), 1);
        let i = changes[0];
        assert_eq!(before[i], b'B');
        match turn {
            0 => assert_eq!(after[i], b'W'),
            1 => assert_eq!(after[i], b'R'),
            _ => {
                assert!(
                    matches!(after[i], b'R' | b'W')
                        && neighbors(i, side).any(|j| before[j] == after[i])
                );
            }
        }
        before = after;
    }
    assert_eq!(model.run(1, &mut rng), 1);
    let after = model.grid().rows().concat().into_bytes();
    assert!(after.contains(&b'U'));
    for (i, &v) in after.iter().enumerate() {
        if v == b'U' {
            assert!(neighbors(i, side).any(|j| before[i] != before[j] && after[j] == b'U'));
        } else {
            assert_eq!(v, before[i]);
        }
    }
}

fn neighbors(i: usize, side: usize) -> impl Iterator<Item = usize> {
    [
        (i % side).checked_sub(1).map(|_| i - 1),
        (i % side + 1 < side).then_some(i + 1),
        i.checked_sub(side),
        (i / side + 1 < side).then_some(i + side),
    ]
    .into_iter()
    .flatten()
}

#[test]
fn ten_buildings_have_distinct_ground_plans_before_coloring()
-> Result<(), Box<dyn std::error::Error>> {
    let mut plans = HashSet::new();
    for seed in 0..10 {
        let mut model = Model::from_xml_3d(include_str!("../models/Courtyard.xml"), 25, 25, 54)?;
        if model.grid().layers().flatten().filter(|&&v| v != 0).count() != 1 {
            return Err("building must start with only one seed voxel".into());
        }
        if model.run(1000, &mut StdRng::seed_from_u64(seed)) == 1000 {
            return Err("building did not finish".into());
        }
        let layers: Vec<_> = model.grid().layers().collect();
        if layers
            .iter()
            .any(|layer| layer.contains(&1) || layer.contains(&2))
        {
            return Err("unfinished roof sites".into());
        }
        let plan: Vec<_> = layers[0].iter().map(|&v| v != 0).collect();
        // Count real floor plates at bay centres, not signs or the transmitter mast.
        let mut storeys = HashSet::new();
        for y in (2..25).step_by(5) {
            for x in (2..25).step_by(5) {
                let i = y * 25 + x;
                if !plan[i] {
                    continue;
                }
                let plates = layers
                    .iter()
                    .skip(2)
                    .step_by(3)
                    .take_while(|layer| {
                        (y - 2..=y + 2).all(|row| {
                            layer[row * 25 + x - 2..=row * 25 + x + 2]
                                .iter()
                                .all(|&v| v == 3)
                        })
                    })
                    .count();
                storeys.insert(plates);
            }
        }
        if storeys.len() < 3 || storeys.iter().max().copied().unwrap_or(0) < 9 {
            return Err(
                format!("missing high-rise or setbacks at seed {seed}: {storeys:?}").into(),
            );
        }
        if plan.iter().filter(|&&v| v).count() <= 9 || !plans.insert(plan) {
            return Err(format!("missing growth or repeated footprint at seed {seed}").into());
        }
    }
    Ok(())
}
