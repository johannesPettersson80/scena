use super::{
    AnimationError, AssetError, BuildError, Error, ImportError, InstantiateError, LookupError,
    PrepareError, RenderError,
};

impl From<BuildError> for Error {
    fn from(error: BuildError) -> Self {
        Self::Build(error)
    }
}

impl From<AssetError> for Error {
    fn from(error: AssetError) -> Self {
        Self::Asset(error)
    }
}

impl From<ImportError> for Error {
    fn from(error: ImportError) -> Self {
        Self::Import(error)
    }
}

impl From<AnimationError> for Error {
    fn from(error: AnimationError) -> Self {
        Self::Animation(error)
    }
}

impl From<InstantiateError> for Error {
    fn from(error: InstantiateError) -> Self {
        Self::Instantiate(error)
    }
}

impl From<PrepareError> for Error {
    fn from(error: PrepareError) -> Self {
        Self::Prepare(error)
    }
}

impl From<RenderError> for Error {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<LookupError> for Error {
    fn from(error: LookupError) -> Self {
        Self::Lookup(error)
    }
}
