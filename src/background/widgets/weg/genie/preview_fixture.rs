//! Generated QA data uses the production pure geometry, never a real window.
//! This module is test-only; the fixture renderer has no native bridge.

use std::{fs, path::PathBuf};

use super::geometry::{self, Bounds};

#[test]
#[ignore = "explicitly exports synthetic geometry data into the repository target/qa directory"]
fn export_owned_genie_geometry_fixture() {
    let source = Bounds {
        x: 180.0,
        y: 120.0,
        width: 560.0,
        height: 350.0,
    };
    let targets = [
        (
            "bottom",
            Bounds {
                x: 615.0,
                y: 650.0,
                width: 52.0,
                height: 52.0,
            },
        ),
        (
            "top",
            Bounds {
                x: 255.0,
                y: 30.0,
                width: 52.0,
                height: 52.0,
            },
        ),
        (
            "left",
            Bounds {
                x: 34.0,
                y: 340.0,
                width: 52.0,
                height: 52.0,
            },
        ),
        (
            "right",
            Bounds {
                x: 870.0,
                y: 250.0,
                width: 52.0,
                height: 52.0,
            },
        ),
    ];
    let cases: Vec<_> = targets.into_iter().map(|(name, target)| {
        let side = geometry::direction(source, target);
        let horizontal = matches!(side, geometry::Direction::Left | geometry::Direction::Right);
        let axis_length = if horizontal { source.width as i32 } else { source.height as i32 };
        let count = (axis_length as usize).min(256);
        let frames: Vec<_> = (0..=100).map(|step| {
            let t = step as f64 / 100.0;
            let strips: Vec<_> = (0..count).map(|index| {
                let begin = index as i32 * axis_length / count as i32;
                let end = (index + 1) as i32 * axis_length / count as i32;
                let destination = geometry::strip(source, target, side, t, index, count);
                let crop = if horizontal { [begin, 0, end - begin, source.height as i32] }
                    else { [0, begin, source.width as i32, end - begin] };
                assert!(destination.width > 0.0 && destination.height > 0.0);
                [crop[0] as f64, crop[1] as f64, crop[2] as f64, crop[3] as f64,
                    destination.x, destination.y, destination.width, destination.height]
            }).collect();
            serde_json::json!({ "progress": t, "strips": strips })
        }).collect();
        serde_json::json!({ "name": name, "target": [target.x, target.y, target.width, target.height], "frames": frames })
    }).collect();
    let fixture = serde_json::json!({
        "scope": "synthetic geometry only; no native capture, source HWND, or Windows minimize",
        "generator": "src/background/widgets/weg/genie/geometry.rs",
        "durationMs": 420,
        "neckPhaseEnd": 0.42,
        "canvas": [960, 760],
        "source": [source.x, source.y, source.width, source.height],
        "cases": cases,
    });
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let output = root.join("target").join("qa").join("genie-geometry.json");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, serde_json::to_vec(&fixture).unwrap()).unwrap();
    println!("GENIE_GEOMETRY_FIXTURE {}", output.display());
}
