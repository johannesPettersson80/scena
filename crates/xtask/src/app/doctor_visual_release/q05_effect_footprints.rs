use crate::app::prelude::*;

const RULE: &str = "Q05-EFFECT-FOOTPRINTS";
const MODE: &str = "quadrant-mean-rgba-v1";
const EFFECT_FIXTURES: &[&str] = &[
    "direct-lights-pbr",
    "shadowed-directional-light",
    "ibl-environment",
    "anti-aliasing-on-off",
    "bloom-on-off",
    "ssao-contact-on-off",
    "oit-overlap-order-invariance",
    "clipping-half-space",
];

pub(crate) fn check_q05_effect_footprint_contracts(root: &Path, findings: &mut Vec<Finding>) {
    let fixture_rel = "tests/visual/fixtures/m2-headless-core.toml";
    let reference_rel = "tests/visual/references/m2-headless-core.toml";
    let proof_rel = "tests/m2_visual_proof.rs";
    let Some(fixture) = read_required(root, findings, fixture_rel) else {
        return;
    };
    let Some(reference) = read_required(root, findings, reference_rel) else {
        return;
    };
    let Some(proof) = read_required(root, findings, proof_rel) else {
        return;
    };

    let fixture_mode = declared_mode(&fixture);
    let reference_mode = declared_mode(&reference);
    if fixture_mode.as_deref() != Some(MODE)
        || reference_mode.as_deref() != Some(MODE)
        || fixture_mode != reference_mode
    {
        findings.push(Finding::new(
            RULE,
            format!(
                "fixture/reference modes must both equal {MODE}; fixture={fixture_mode:?} reference={reference_mode:?}"
            ),
        ));
    }
    for forbidden in [
        "sampled-rgba",
        "rgba_hash",
        "center_rgba",
        "left_mid_rgba",
        "right_mid_rgba",
    ] {
        if fixture.contains(forbidden) || reference.contains(forbidden) || proof.contains(forbidden)
        {
            findings.push(Finding::new(
                RULE,
                format!("retired exact/three-sample token remains: {forbidden}"),
            ));
        }
    }

    for name in EFFECT_FIXTURES {
        let Some(fixture_block) = table_block(&fixture, "[[fixture]]", name) else {
            findings.push(Finding::new(
                RULE,
                format!("{fixture_rel} is missing paired effect fixture {name}"),
            ));
            continue;
        };
        for required in [
            "proof_class = \"paired-effect-footprint\"",
            "pair = ",
            "spatial_mask = [",
        ] {
            if !fixture_block.contains(required) {
                findings.push(Finding::new(
                    RULE,
                    format!("effect fixture {name} is missing {required}"),
                ));
            }
        }

        let Some(reference_block) = table_block(&reference, "[[reference]]", name) else {
            findings.push(Finding::new(
                RULE,
                format!("{reference_rel} is missing tolerant reference {name}"),
            ));
            continue;
        };
        for required in [
            "max_abs_diff = 3",
            "top_left_mean_rgba = [",
            "top_right_mean_rgba = [",
            "bottom_left_mean_rgba = [",
            "bottom_right_mean_rgba = [",
            "quadrant_nonblack = [",
        ] {
            if !reference_block.contains(required) {
                findings.push(Finding::new(
                    RULE,
                    format!("reference {name} is missing {required}"),
                ));
            }
        }
    }

    for required in [
        "struct EffectPair",
        "struct PixelMask",
        "fn effect_pair_failures",
        "fn quadrant_reference_matches",
        "fn fixture_reference_mode",
        "fn reference_mode",
        "fn q05_reference_oracle_rejects_quadrant_corruption_outside_legacy_samples",
        "fn q05_effect_footprint_masks_reject_erased_effect_regions",
    ] {
        if !proof.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{proof_rel} is missing {required}"),
            ));
        }
    }
}

fn read_required(root: &Path, findings: &mut Vec<Finding>, relative: &str) -> Option<String> {
    match fs::read_to_string(root.join(relative)) {
        Ok(text) => Some(text),
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {relative}: {error}"),
            ));
            None
        }
    }
}

fn declared_mode(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("reference_mode = ")
            .and_then(|value| value.trim().strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .map(str::to_owned)
    })
}

fn table_block<'a>(text: &'a str, table: &str, name: &str) -> Option<&'a str> {
    let table_start = text.find(&format!("{table}\nname = \"{name}\""))?;
    let rest = &text[table_start..];
    let next = rest[table.len()..]
        .find(table)
        .map(|offset| offset + table.len())
        .unwrap_or(rest.len());
    Some(&rest[..next])
}
