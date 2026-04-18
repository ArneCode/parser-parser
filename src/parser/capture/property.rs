use super::match_result::MatchResult;

#[derive(Clone, Copy)]
pub struct BindDebugInfo {
    pub property_name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

pub trait Property<Value, MRes> {
    fn put_in_result(&self, result: &mut MRes, value: Value, debug: Option<BindDebugInfo>);
    fn bind_result(&self, value: Value) -> super::bound::BoundValue<Value, Self>
    where
        Self: Sized,
        Self: Clone,
    {
        super::bound::BoundValue::new(value, self.clone(), None)
    }
    fn bind_result_with_debug(
        &self,
        value: Value,
        debug: BindDebugInfo,
    ) -> super::bound::BoundValue<Value, Self>
    where
        Self: Sized,
        Self: Clone,
    {
        super::bound::BoundValue::new(value, self.clone(), Some(debug))
    }
}

#[derive(Clone, Copy)]
pub struct SingleProperty<F> {
    setter: F,
}

impl<F> SingleProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for SingleProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Single) -> &mut Option<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.single());
        if property_slot.is_some() {
            if let Some(debug) = debug {
                panic!(
                    "SingleProperty '{}' already set (bind! at {}:{}:{})",
                    debug.property_name, debug.file, debug.line, debug.column
                );
            }
            panic!("SingleProperty already set");
        }
        *property_slot = Some(value);
    }
}

#[derive(Clone, Copy)]
pub struct MultipleProperty<F> {
    setter: F,
}

impl<F> MultipleProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for MultipleProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Multiple) -> &mut Vec<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.multiple());
        property_slot.push(value);
    }
}

#[derive(Clone, Copy)]
pub struct OptionalProperty<F> {
    setter: F,
}

impl<F> OptionalProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for OptionalProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Optional) -> &mut Option<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.optional());
        if property_slot.is_some() {
            if let Some(debug) = debug {
                panic!(
                    "OptionalProperty '{}' already set (bind! at {}:{}:{})",
                    debug.property_name, debug.file, debug.line, debug.column
                );
            }
            panic!("OptionalProperty already set");
        }
        *property_slot = Some(value);
    }
}
