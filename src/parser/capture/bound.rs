use super::property::{BindDebugInfo, Property};

/// A value pending insertion into a match-result bucket `MRes`.
pub trait BoundResult<MRes> {
    /// Write this capture into `result`.
    fn put_in_result(self, result: &mut MRes);
    /// Remove this capture from the result.
    fn remove_from_result(&self, result: &mut MRes);
    /// Write a boxed capture into `result`.
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes);
}

/// Pair of a captured value and the [`Property`] that knows how to store it.
pub struct BoundValue<Value, Prop> {
    pub(super) value: Value,
    pub(super) property: Prop,
    pub(super) debug: Option<BindDebugInfo>,
}

impl<Value, Prop> BoundValue<Value, Prop> {
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
    Prop: Property<Value, MRes>,
{
    fn put_in_result(self, result: &mut MRes) {
        self.property.put_in_result(result, self.value, self.debug);
    }

    fn remove_from_result(&self, result: &mut MRes) {
        self.property.remove_from_result(result, self.debug);
    }

    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes) {
        (*self).put_in_result(result)
    }
}
