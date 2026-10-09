//! Immutable build identity supplied by the platform, separate from saved settings.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildInfo {
    pub version: &'static str,
    pub board: &'static str,
    pub revision: &'static str,
    pub development: bool,
    pub dirty: bool,
}

impl BuildInfo {
    /// Identity for a harness or a platform that does not supply metadata.
    pub const UNKNOWN: Self = Self {
        version: "unknown",
        board: "unknown",
        revision: "unknown",
        development: true,
        dirty: false,
    };
}
