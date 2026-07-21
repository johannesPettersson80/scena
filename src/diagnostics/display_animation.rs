use std::fmt;

use super::AnimationError;

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClipNotFound { name, candidates } => {
                write!(
                    formatter,
                    "imported scene has no animation clip named '{name}'"
                )?;
                if !candidates.is_empty() {
                    write!(formatter, "; nearest candidates: {}", candidates.join(", "))?;
                }
                Ok(())
            }
            Self::InvalidClip { reason } => {
                write!(formatter, "animation clip is invalid: {reason}")
            }
            Self::MixerNotFound(_) => write!(formatter, "animation mixer key does not exist"),
            Self::StaleMixer(_) => write!(
                formatter,
                "animation mixer is stale because its source import was replaced"
            ),
        }
    }
}
