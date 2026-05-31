//! Parse mode: compile-time flags for error recovery and committed rules.
//!
//! Only the top-level driver selects [`Emit`] with `IS_IN_ERROR_RECOVERY` via `parse::<Emit<…>>`;
//! nested code forwards the same `M` unless a combinator changes mode (for example [`CommitMatcher`]
//! uses [`Mode::Committed`] after `commit_on` succeeds).

// For now only do Emit, maybe later do check also?
pub(crate) struct Emit<const IS_IN_ERROR_RECOVERY: bool, const IS_IN_COMMITTED: bool>;

pub(crate) trait Mode {
    const IS_IN_ERROR_RECOVERY: bool;
    const IS_IN_COMMITTED: bool;
    /// Mode for matchers/parsers after `commit_on` succeeds.
    type Committed: Mode;
}

impl<const IS_IN_ERROR_RECOVERY: bool, const IS_IN_COMMITTED: bool> Mode
    for Emit<IS_IN_ERROR_RECOVERY, IS_IN_COMMITTED>
{
    const IS_IN_ERROR_RECOVERY: bool = IS_IN_ERROR_RECOVERY;
    const IS_IN_COMMITTED: bool = IS_IN_COMMITTED;
    type Committed = Emit<IS_IN_ERROR_RECOVERY, true>;
}

/// Runtime mode for object-safe parser dispatch ([`crate::parser::ParserObjSafe`]).
pub(crate) enum ConcreteMode {
    Emit {
        is_in_error_recovery: bool,
        is_in_committed: bool,
    },
}

impl ConcreteMode {
    pub(crate) fn from_mode<M: Mode>() -> Self {
        Self::Emit {
            is_in_error_recovery: M::IS_IN_ERROR_RECOVERY,
            is_in_committed: M::IS_IN_COMMITTED,
        }
    }
}
