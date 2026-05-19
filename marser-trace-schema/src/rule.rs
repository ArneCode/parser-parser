//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use serde::{Deserialize, Serialize};

/// Call-site metadata for a traced rule (not serialized as a standalone trace value).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleSourceMetadata {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub rule_name: Option<&'static str>,
}

impl RuleSourceMetadata {
    pub const fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self {
            file,
            line,
            column,
            rule_name: None,
        }
    }

    pub const fn with_rule_name(self, rule_name: &'static str) -> Self {
        Self {
            rule_name: Some(rule_name),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleIdentity {
    pub rule_id: u64,
    pub rule_name: Option<String>,
    pub rule_file: String,
    pub rule_line: u32,
    pub rule_column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}
