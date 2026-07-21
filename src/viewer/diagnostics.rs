use crate::assets::Assets;
use crate::diagnostics::Diagnostic;
use crate::render::Renderer;
use crate::scene::Scene;

pub(super) fn combined_viewer_diagnostics(
    setup: &[Diagnostic],
    renderer: &Renderer,
    scene: &Scene,
    assets: &Assets,
) -> Vec<Diagnostic> {
    let mut diagnostics = setup.to_vec();
    diagnostics.extend(renderer.diagnostics().iter().cloned());
    diagnostics.extend(renderer.diagnose_scene_with_assets(scene, assets));
    diagnostics.dedup_by(|left, right| left.code == right.code && left.message == right.message);
    diagnostics
}
