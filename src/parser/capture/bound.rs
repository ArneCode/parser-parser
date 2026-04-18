use super::property::{BindDebugInfo, Property};

pub trait BoundResult<MRes> {
    fn put_in_result(self, result: &mut MRes);
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes);
}

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

    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes) {
        (*self).put_in_result(result)
    }
}
