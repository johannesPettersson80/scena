use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs};
use crate::{Scene, SceneCompositionCheckV1, SceneRecipeExpectV1, SceneRecipeV1};

pub(super) fn composition_clipping_checks(
    recipe: &SceneRecipeV1,
    scene: &Scene,
    expect: Option<&SceneRecipeExpectV1>,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let active_user_planes = scene.clipping_planes().planes().len() as u32;
    let section = scene.section_box();
    let section_active = section.is_some();
    let section_inverted = section.is_some_and(|section| section.inverted());
    let section_plane_count = scene.section_box_planes().map_or(0, |planes| planes.len());

    if !recipe.clipping_planes.is_empty() {
        let expected = recipe
            .clipping_planes
            .iter()
            .filter(|plane| plane.active)
            .count() as u32;
        push_plane_count_check(
            &mut checks,
            "recipe.clipping_planes.active",
            expected,
            active_user_planes,
            "recipe active clipping-plane count matches renderer state",
            "recipe active clipping-plane count does not match renderer state",
        );
    }

    if let Some(section_box) = recipe.section_box.as_ref() {
        push_section_active_check(
            &mut checks,
            "recipe.section_box.active",
            true,
            section_active,
            section_plane_count,
        );
        push_section_inverted_check(
            &mut checks,
            "recipe.section_box.inverted",
            section_box.inverted,
            section_active,
            section_inverted,
            section_plane_count,
        );
    }

    if let Some(expect_clipping) = expect.and_then(|expect| expect.expect_clipping.as_ref()) {
        if let Some(expected) = expect_clipping.active_clipping_planes {
            push_plane_count_check(
                &mut checks,
                "expect_clipping.active_clipping_planes",
                expected,
                active_user_planes,
                "active clipping-plane count satisfies expect_clipping",
                "active clipping-plane count does not satisfy expect_clipping",
            );
        }
        if let Some(expected) = expect_clipping.section_box_active {
            push_section_active_check(
                &mut checks,
                "expect_clipping.section_box_active",
                expected,
                section_active,
                section_plane_count,
            );
        }
        if let Some(expected) = expect_clipping.section_box_inverted {
            push_section_inverted_check(
                &mut checks,
                "expect_clipping.section_box_inverted",
                expected,
                section_active,
                section_inverted,
                section_plane_count,
            );
        }
    }

    checks
}

fn push_plane_count_check(
    checks: &mut Vec<SceneCompositionCheckV1>,
    id: &str,
    expected: u32,
    actual: u32,
    pass_message: &str,
    fail_message: &str,
) {
    let observed = observed_pairs([("expected", json!(expected)), ("actual", json!(actual))]);
    if expected == actual {
        checks.push(checked_check(
            id.to_owned(),
            "clipping_section",
            "clipping_plane_count_satisfied",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (pass_message, "no action needed"),
        ));
    } else {
        checks.push(error_check(
            id.to_owned(),
            "clipping_section",
            "clipping_plane_count_mismatch",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                fail_message,
                "activate the declared clipping planes or update expect_clipping to match the intended render state",
            ),
        ));
    }
}

fn push_section_active_check(
    checks: &mut Vec<SceneCompositionCheckV1>,
    id: &str,
    expected: bool,
    actual: bool,
    section_plane_count: usize,
) {
    let observed = observed_pairs([
        ("expected", json!(expected)),
        ("actual", json!(actual)),
        ("section_plane_count", json!(section_plane_count)),
    ]);
    match (expected, actual) {
        (true, true) => checks.push(checked_check(
            id.to_owned(),
            "clipping_section",
            "section_box_active",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "section box is active in renderer state",
                "no action needed",
            ),
        )),
        (false, false) => checks.push(checked_check(
            id.to_owned(),
            "clipping_section",
            "section_box_inactive",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            ("section box is inactive as expected", "no action needed"),
        )),
        (true, false) => checks.push(error_check(
            id.to_owned(),
            "clipping_section",
            "section_box_missing",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "expected section box is not active in renderer state",
                "restore section_box setup or remove the expectation if clipping is not intended",
            ),
        )),
        (false, true) => checks.push(error_check(
            id.to_owned(),
            "clipping_section",
            "section_box_unexpected_active",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "section box is active but expectation requires it inactive",
                "clear section_box or update expect_clipping if section clipping is intended",
            ),
        )),
    }
}

fn push_section_inverted_check(
    checks: &mut Vec<SceneCompositionCheckV1>,
    id: &str,
    expected: bool,
    section_active: bool,
    actual: bool,
    section_plane_count: usize,
) {
    let observed = observed_pairs([
        ("expected", json!(expected)),
        ("actual", json!(actual)),
        ("section_active", json!(section_active)),
        ("section_plane_count", json!(section_plane_count)),
    ]);
    if !section_active {
        checks.push(error_check(
            id.to_owned(),
            "clipping_section",
            "section_box_missing",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "section box inversion cannot be checked because no section box is active",
                "activate section_box before asserting section_box_inverted",
            ),
        ));
    } else if expected == actual {
        checks.push(checked_check(
            id.to_owned(),
            "clipping_section",
            "section_box_inversion_satisfied",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "section box inversion state satisfies the declared expectation",
                "no action needed",
            ),
        ));
    } else {
        checks.push(error_check(
            id.to_owned(),
            "clipping_section",
            "section_box_inversion_mismatch",
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (
                "section box inversion state does not match the declared expectation",
                "set section_box.inverted to the intended value or update expect_clipping",
            ),
        ));
    }
}
