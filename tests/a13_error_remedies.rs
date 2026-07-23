use scena::{
    AnimationError, AnimationMixerKey, AssetError, Backend, BuildError, Error, InstantiateError,
    LookupError, NodeKey, PrepareError, RenderError, SourceCoordinateSystem,
};
use slotmap::Key;

#[test]
fn every_build_instantiate_and_animation_variant_has_curated_help() {
    let build = [
        BuildError::InvalidTargetSize {
            width: 0,
            height: 1,
        },
        BuildError::AsyncSurfaceRequired {
            backend: Backend::NativeSurface,
        },
        BuildError::CreateSurface {
            backend: Backend::NativeSurface,
        },
        BuildError::NoAdapter {
            backend: Backend::HeadlessGpu,
        },
        BuildError::RequestDevice {
            backend: Backend::HeadlessGpu,
        },
        BuildError::SurfaceUnsupported {
            backend: Backend::NativeSurface,
        },
        BuildError::UnsupportedBackend {
            backend: Backend::WebGpu,
        },
    ];
    assert_complete(build.iter());

    let instantiate = [
        InstantiateError::InvalidChildIndex {
            parent: 0,
            child: 1,
        },
        InstantiateError::InvalidSkinIndex { node: 0, skin: 1 },
        InstantiateError::InvalidSkinJointIndex { skin: 0, joint: 1 },
        InstantiateError::InvalidAnimationClip {
            name: None,
            reason: "bad".into(),
        },
        InstantiateError::InvalidAnchorExtras {
            node: "n".into(),
            reason: "bad".into(),
        },
        InstantiateError::InvalidConnectorExtras {
            node: "n".into(),
            reason: "bad".into(),
        },
        InstantiateError::CyclicNodeGraph { node: 0 },
        InstantiateError::MultipleNodeParents {
            node: 0,
            first_parent: 1,
            second_parent: 2,
        },
        InstantiateError::StaleReplacementImport,
        InstantiateError::ForeignReplacementImport,
        InstantiateError::MissingReplacementRoot {
            root: NodeKey::null(),
        },
        InstantiateError::UnsupportedCoordinateSystem {
            coordinate_system: SourceCoordinateSystem::ZUpRightHanded,
            reason: "bad".into(),
        },
    ];
    assert_complete(instantiate.iter());

    let animation = [
        AnimationError::ClipNotFound {
            name: "missing".into(),
            candidates: vec![],
        },
        AnimationError::InvalidClip {
            reason: "bad".into(),
        },
        AnimationError::MixerNotFound(AnimationMixerKey::null()),
        AnimationError::StaleMixer(AnimationMixerKey::null()),
    ];
    assert_complete(animation.iter());
}

#[test]
fn import_and_top_level_errors_delegate_help_and_structured_diagnostics() {
    let import = scena::ImportError::Asset(AssetError::NotFound {
        path: "missing".into(),
    });
    assert_eq!(
        import.help(),
        match &import {
            scena::ImportError::Asset(error) => error.help(),
            _ => unreachable!(),
        }
    );
    assert!(!import.diagnostic().help.is_empty());

    let errors = [
        Error::Build(BuildError::InvalidTargetSize {
            width: 0,
            height: 0,
        }),
        Error::Asset(AssetError::NotFound {
            path: "missing".into(),
        }),
        Error::Import(import),
        Error::Instantiate(InstantiateError::CyclicNodeGraph { node: 0 }),
        Error::Prepare(PrepareError::InvalidTargetSize {
            width: 0,
            height: 0,
        }),
        Error::Render(RenderError::NoActiveCamera),
        Error::Lookup(LookupError::NoActiveCamera),
        Error::Animation(AnimationError::InvalidClip {
            reason: "bad".into(),
        }),
    ];
    for error in errors {
        let diagnostic = error.diagnostic();
        assert!(!error.help().is_empty());
        assert!(!diagnostic.code.is_empty());
        assert_eq!(diagnostic.message, error.to_string());
        assert_eq!(diagnostic.help, error.help());
        assert!(diagnostic.context.contains_key("family"));
    }
}

fn assert_complete<'a, T>(errors: impl IntoIterator<Item = &'a T>)
where
    T: std::fmt::Display + 'a,
    &'a T: ErrorRemedy,
{
    for error in errors {
        assert!(!error.remedy().is_empty(), "missing help for {error}");
        let diagnostic = error.structured();
        assert_eq!(diagnostic.message, error.to_string());
        assert!(!diagnostic.code.is_empty());
        assert!(!diagnostic.help.is_empty());
    }
}

trait ErrorRemedy {
    fn remedy(self) -> &'static str;
    fn structured(self) -> scena::ErrorDiagnostic;
}

impl ErrorRemedy for &BuildError {
    fn remedy(self) -> &'static str {
        self.help()
    }
    fn structured(self) -> scena::ErrorDiagnostic {
        self.diagnostic()
    }
}

impl ErrorRemedy for &InstantiateError {
    fn remedy(self) -> &'static str {
        self.help()
    }
    fn structured(self) -> scena::ErrorDiagnostic {
        self.diagnostic()
    }
}

impl ErrorRemedy for &AnimationError {
    fn remedy(self) -> &'static str {
        self.help()
    }
    fn structured(self) -> scena::ErrorDiagnostic {
        self.diagnostic()
    }
}
