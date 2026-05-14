use crate::parser::capture::MatchResult;

use super::property::{BindDebugInfo, Property};

/// A value pending insertion into a match-result bucket `MRes`.
pub trait BoundResult<MRes>
where
    MRes: MatchResult,
{
    /// Write this capture into `result`.
    fn put_in_result(self, result: &mut MRes);
    /// Remove this capture from the result.
    fn remove_from_result(&self, result: &mut MRes);
    /// Write a boxed capture into `result`.
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes);
    /// Insert a reference to this capture into the corresponding slot of `snapshot`.
    ///
    /// The data lifetime of the inserted reference is `'a`, the lifetime of `&self`.
    fn put_ref_in_snapshot<'a>(&'a self, snapshot: &mut MRes::Snapshot<'a>);
}

/// Pair of a captured value and the [`Property`] that knows how to store it.
pub struct BoundValue<Value, Prop> {
    pub(super) value: Value,
    pub(super) property: Prop,
    pub(super) debug: Option<BindDebugInfo>,
}

impl<Value, Prop> BoundValue<Value, Prop> {
    #[inline]
    pub(super) fn new(value: Value, property: Prop, debug: Option<BindDebugInfo>) -> Self {
        Self {
            value,
            property,
            debug,
        }
    }
}

impl<Value, MRes, Prop> BoundResult<MRes> for BoundValue<Value, Prop>
where
    MRes: MatchResult,
    Prop: Property<Value, MRes>,
{
    #[inline]
    fn put_in_result(self, result: &mut MRes) {
        self.property.put_in_result(result, self.value, self.debug);
    }

    #[inline]
    fn remove_from_result(&self, result: &mut MRes) {
        self.property.remove_from_result(result, self.debug);
    }

    #[inline]
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes) {
        (*self).put_in_result(result)
    }

    #[inline]
    fn put_ref_in_snapshot<'a>(&'a self, snapshot: &mut MRes::Snapshot<'a>) {
        self.property.put_ref_in_snapshot(snapshot, &self.value);
    }
}
