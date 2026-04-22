use super::match_result::MatchResult;

/// Source location and name for debugging double-bind panics.
#[derive(Clone, Copy)]
pub struct BindDebugInfo {
    /// Name of the binding (from `bind!` / macro expansion).
    pub property_name: &'static str,
    /// File where the bind was created.
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

/// Describes how to write a captured `Value` into the aggregate result `MRes`.
pub trait Property<Value, MRes> {
    /// Store `value` into `result`, optionally using `debug` for panic messages on conflict.
    fn put_in_result(&self, result: &mut MRes, value: Value, debug: Option<BindDebugInfo>);
    /// Remove `value` from `result`.
    fn remove_from_result(&self, result: &mut MRes, debug: Option<BindDebugInfo>);
    /// Wrap `value` with this property for deferred insertion into `MRes`.
    fn bind_result(&self, value: Value) -> super::bound::BoundValue<Value, Self>
    where
        Self: Sized,
        Self: Clone,
    {
        super::bound::BoundValue::new(value, self.clone(), None)
    }
    /// Like [`bind_result`](Self::bind_result) but attaches [`BindDebugInfo`] for diagnostics.
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

/// Property that writes at most once into an `Option` field (panics if set twice).
#[derive(Clone, Copy)]
pub struct SingleProperty<F> {
    setter: F,
}

impl<F> SingleProperty<F> {
    /// `setter` selects the `Option` slot inside the “single” capture bucket.
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

    fn remove_from_result(&self, result: &mut MRes, debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.single());
        if property_slot.is_some() {
            *property_slot = None;
        }else{
            panic!("Trying to remove a value that was not set");
        }
    }
}

/// Property that appends each capture into a `Vec`.
#[derive(Clone, Copy)]
pub struct MultipleProperty<F> {
    setter: F,
}

impl<F> MultipleProperty<F> {
    /// `setter` selects the `Vec` field inside the “multiple” capture bucket.
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

    fn remove_from_result(&self, result: &mut MRes, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.multiple());
        if property_slot.pop().is_none() {
            panic!("Trying to remove a value that was not set");
        }
    }
}

/// Property for an optional capture stored in `Option` (panics if set twice).
#[derive(Clone, Copy)]
pub struct OptionalProperty<F> {
    setter: F,
}

impl<F> OptionalProperty<F> {
    /// `setter` selects the `Option` slot inside the “optional” capture bucket.
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

    fn remove_from_result(&self, result: &mut MRes, debug: Option<BindDebugInfo>) {
        let property_slot = (self.setter)(result.optional());
        if property_slot.is_some() {
            *property_slot = None;
        }else{
            panic!("Trying to remove a value that was not set");
        }
    }
}
