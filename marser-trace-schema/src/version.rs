//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.
//!
//! On-disk `trace_version` policy for JSON traces.

/// Version written in the `{ "trace_version", "nodes", "source_text?" }` JSON envelope.
pub const SCHEMA_VERSION: u32 = 3;

/// Lowest `trace_version` value this crate accepts when loading JSON.
pub const SUPPORTED_TRACE_VERSION_MIN: u32 = 2;

/// Highest `trace_version` value this crate accepts when loading JSON.
pub const SUPPORTED_TRACE_VERSION_MAX: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedTraceVersion {
    pub found: u32,
    pub min: u32,
    pub max: u32,
}

impl std::fmt::Display for UnsupportedTraceVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported trace_version {} (supported {}..={})",
            self.found, self.min, self.max
        )
    }
}

impl std::error::Error for UnsupportedTraceVersion {}

/// Validates `trace_version` from a JSON object payload.
///
/// `None` is treated as version [`SCHEMA_VERSION`] for backward compatibility.
pub fn check_trace_version(version: Option<u32>) -> Result<(), UnsupportedTraceVersion> {
    let v = version.unwrap_or(SCHEMA_VERSION);
    if v < SUPPORTED_TRACE_VERSION_MIN || v > SUPPORTED_TRACE_VERSION_MAX {
        return Err(UnsupportedTraceVersion {
            found: v,
            min: SUPPORTED_TRACE_VERSION_MIN,
            max: SUPPORTED_TRACE_VERSION_MAX,
        });
    }
    Ok(())
}
