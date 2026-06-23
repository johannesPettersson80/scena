pub(super) fn compile_interaction_expectation(
    expect: &scena::SceneRecipeExpectV1,
    manifest: &scena::SceneRecipeBuildV1,
    capture: &scena::CaptureRgba8,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> scena::InteractionExpectationV1 {
    let mut steps = Vec::new();
    for pick in &expect.expect_pick {
        let handle = match super::resolve_target_handle(&pick.target, manifest) {
            Ok(handle) => handle,
            Err(message) => {
                super::push_reason(
                    reasons,
                    "target_not_found",
                    "interaction",
                    Some(pick.id.clone()),
                    Vec::new(),
                    message,
                );
                continue;
            }
        };
        steps.push(scena::InteractionStepExpectationV1 {
            action: "pick".to_owned(),
            x_css_px: pick.x_css_px as f32,
            y_css_px: pick.y_css_px as f32,
            coordinate_space: "css".to_owned(),
            expect_hit: Some(true),
            expected_handle: Some(handle),
            expect_hover: None,
            expect_selection: None,
            expected_events: vec!["pick".to_owned()],
        });
    }
    scena::InteractionExpectationV1 {
        schema: scena::INTERACTION_EXPECTATION_SCHEMA_V1.to_owned(),
        viewport: scena::InteractionViewportV1 {
            width_css_px: capture.descriptor.width as f32,
            height_css_px: capture.descriptor.height as f32,
            device_pixel_ratio: 1.0,
        },
        steps,
    }
}

pub(super) fn run_interaction_verification(
    host: &mut scena::SceneHostCore,
    expectation: scena::InteractionExpectationV1,
) -> Result<scena::InteractionVerificationReportV1, String> {
    let artifacts = scena::InteractionVerificationArtifactsV1::from_viewport(expectation.viewport);
    let _ = host.drain_events();
    let mut steps = Vec::with_capacity(expectation.steps.len());
    for (index, step) in expectation.steps.iter().enumerate() {
        let coordinates = scena::InteractionCoordinatesV1::from_step(step, expectation.viewport)?;
        let handle = host
            .pick(coordinates.x_css_px, coordinates.y_css_px)
            .map_err(|error| format!("recipe interaction pick failed: {error}"))?;
        let events = host
            .drain_events()
            .iter()
            .map(scena::host_event_kind_name)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        steps.push(scena::InteractionStepReportV1 {
            index,
            action: step.action.clone(),
            coordinates,
            expected: scena::InteractionStepExpectedV1::from(step),
            observed: scena::InteractionStepObservedV1 {
                hit: handle.is_some(),
                handle,
                hover_handle: host.hover_handle(),
                selection_handle: host.primary_selection_handle(),
                events,
            },
        });
    }
    Ok(scena::InteractionVerificationReportV1::from_steps(
        artifacts, steps,
    ))
}
