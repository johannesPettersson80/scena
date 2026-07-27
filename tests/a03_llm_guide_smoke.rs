#![cfg(all(not(target_arch = "wasm32"), feature = "agent"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use image::GenericImageView;

const GUIDE: &str = include_str!("../docs/guides/llm-app-builder.md");
const START: &str = "<!-- SCENA_CANONICAL_AGENT_SMOKE_BEGIN -->";
const END: &str = "<!-- SCENA_CANONICAL_AGENT_SMOKE_END -->";

#[test]
fn camera_behavior_guide_pins_easy_path_reports_and_demo_rule() {
    assert!(
        GUIDE.contains(
            "scena photo render model.glb --out hero.png --report hero.report.json --emit-recipe hero.resolved.recipe.json"
        ),
        "LLM guide must route product/model hero stills through the photo-render easy path",
    );
    assert!(
        GUIDE.contains("\"photo\": {\n    \"intent\": \"camera_behavior\""),
        "LLM guide must show the recipe-native photo.intent path",
    );
    assert!(
        GUIDE.contains("photo_report.exposure_report"),
        "LLM guide must tell agents where exposure diagnostics live",
    );
    assert!(
        GUIDE.contains("suggested_compensation_ev"),
        "LLM guide must tell agents how to read suggested exposure compensation",
    );
    assert!(
        GUIDE.contains("public demo hero"),
        "LLM guide must pin the no-hand-tuned-overrides rule for public demo hero renders",
    );
    assert!(
        GUIDE.contains(
            "no hand-tuned camera, exposure, focus, floor, grid, or background overrides"
        ),
        "LLM guide must keep the public demo hero on the intent path instead of per-shot constants",
    );
}

#[test]
fn canonical_agent_guide_block_runs_from_a_clean_directory() {
    let scena = std::env::var_os("SCENA_A03_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_scena")));
    let scena = scena
        .canonicalize()
        .unwrap_or_else(|error| panic!("{} resolves: {error}", scena.display()));
    let bin_dir = scena.parent().expect("scena binary has parent");
    let work = unique_temp_dir();
    fs::create_dir(&work).expect("clean guide directory creates");
    let block = canonical_block();
    assert!(
        GUIDE.contains("cargo build --release --bin scena --features agent"),
        "local-checkout performance guidance must use a release binary",
    );
    assert!(
        block.contains("recipe render \"$RECIPE\" --timings"),
        "canonical agent render must request observational stage timings",
    );
    assert!(block.contains("mkdir -p target/scena-agent"));
    assert!(!block.contains("primitive_scene"));
    assert!(
        !block.contains("--introspect"),
        "introspection is the default and the canonical guide should not require the compatibility flag"
    );

    let path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(bin_dir.to_path_buf()).chain(std::env::split_paths(&path)),
    )
    .expect("guide PATH joins");
    let output = Command::new("bash")
        .args(["-euo", "pipefail", "-c", &block])
        .current_dir(&work)
        .env("PATH", path)
        .output()
        .expect("canonical guide shell runs");
    assert!(
        output.status.success(),
        "canonical guide failed: status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    for (relative, schema) in [
        ("scene-recipe.schema.json", "scena.schema_entry.v1"),
        ("templates.json", "scena.agent_template_catalog.v1"),
        (
            "primitive-scene.manifest.json",
            "scena.agent_smoke_template.v1",
        ),
        ("validation.json", "scena.scene_recipe_validation.v1"),
        ("build.json", "scena.recipe_build_result.v1"),
        ("render.json", "scena.render_introspection.v1"),
    ] {
        let value = read_json(&work.join("target/scena-agent").join(relative));
        assert_eq!(value["schema"], schema, "file={relative}");
        if matches!(relative, "validation.json" | "build.json" | "render.json") {
            assert_eq!(value["ok"], true, "file={relative} value={value:#}");
        }
        if relative == "render.json" {
            assert_eq!(value["timings"]["status"], "measured");
            for field in ["prepare_ms", "render_ms", "capture_ms", "total_ms"] {
                assert!(
                    value["timings"][field].is_u64(),
                    "render timing {field} must be measured: {value:#}",
                );
            }
        }
    }

    let frame = image::open(work.join("target/scena-agent/frame.png"))
        .expect("canonical guide frame decodes");
    assert!(frame.width() > 0 && frame.height() > 0);
    assert!(
        frame
            .pixels()
            .any(|(_, _, pixel)| pixel.0[..3].iter().any(|channel| *channel != 0)),
        "canonical guide frame must contain a visible result"
    );
    fs::remove_dir_all(work).expect("guide temp directory removes");
}

fn canonical_block() -> String {
    let marked = GUIDE
        .split_once(START)
        .expect("guide has canonical start marker")
        .1
        .split_once(END)
        .expect("guide has canonical end marker")
        .0;
    marked
        .trim()
        .strip_prefix("```bash")
        .and_then(|block| block.strip_suffix("```"))
        .expect("canonical block is one bash fence")
        .trim()
        .to_owned()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(path).unwrap_or_else(|error| panic!("{} reads: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("{} is JSON: {error}", path.display()))
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("scena-a03-guide-{}-{nonce}", std::process::id()))
}
