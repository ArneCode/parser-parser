//! Implementation of the cache used to enable Packrat-parsing using the memoized parser.
//!
//! # Why unsafe is used
//! We want to be able to memoize different output types of parsers. I do not want the Parsers to hold mutable state, so we need
//! to store the memoized valued seperately. So we need some sort of storage which can store valued of different types and retrieve them.
//! We cannot use dyn Any because Any requires our values to be static but we want to allow for zero-copy parsing.
//! Existing libraries I could find either require that Values are static or some trait of that library is defined on them.
//!
use std::{collections::HashMap, marker::PhantomData};

type MemoTable<T> = Vec<T>;
pub(crate) type ParserId = usize;
type EntryId = usize;
struct ErasedMemoTable<'src> {
    ptr: *mut (),
    drop_fn: unsafe fn(*mut ()),
    _marker: PhantomData<&'src mut ()>,
}

impl<'src> Drop for ErasedMemoTable<'src> {
    fn drop(&mut self) {
        unsafe { (self.drop_fn)(self.ptr) };
    }
}

/// drops ptr as Box<MemoTable<T>>
/// # Safety
/// - can only be called with same T as ptr has been created with.
///
/// This function is only used to drop an ErasedMemoryTable, where it is stored bound to the same T
/// that the ErasedMemoryTable has been created with.
unsafe fn drop_erased_memo_table<T>(ptr: *mut ()) {
    let ptr: *mut MemoTable<T> = ptr.cast();
    let table = unsafe { Box::from_raw(ptr) };
    drop(table);
}

impl<'src> ErasedMemoTable<'src> {
    fn new<T: 'src>() -> Self {
        let boxed = Box::new(MemoTable::<T>::new());
        let ptr: *mut () = Box::into_raw(boxed).cast();

        Self {
            ptr,
            drop_fn: drop_erased_memo_table::<T>,
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// - can only be called with same T as self has been created with.
    ///
    /// Only ever called
    unsafe fn get_unerased_table<'a, T: 'src>(&'a self) -> &'a MemoTable<T> {
        let ptr: *mut MemoTable<T> = self.ptr.cast();
        unsafe { &*ptr }
    }

    /// # Safety
    /// - can only be called with same T as self has been created with.
    unsafe fn get_unerased_table_mut<'a, T: 'src>(&'a mut self) -> &'a mut MemoTable<T> {
        let ptr: *mut MemoTable<T> = self.ptr.cast();
        unsafe { &mut *ptr }
    }

    /// # Safety
    /// - can only be called with same T as self has been created with.
    unsafe fn get_entry<'a, T: 'src>(&'a self, entry_id: EntryId) -> &'a T {
        let table = unsafe { self.get_unerased_table::<T>() };
        &table[entry_id]
    }

    /// # Safety
    /// - can only be called with same T as self has been created with.
    unsafe fn add_entry<T: 'src>(&mut self, entry: T) -> EntryId {
        let table = unsafe { self.get_unerased_table_mut::<T>() };
        let entry_id = table.len();
        table.push(entry);
        entry_id
    }
}

pub(crate) struct Cache<'src> {
    // Potential improvement: Because should not be many parsers that are memoized we could store this as a Vec
    // where every pos corresponds to a Vec<(parser_id, (table_id, entry_id))
    // then for lookup just need to look through a small number of entries at pos and compare parser_id.
    // Pegen does something similar but stores linked list at every pos

    //TODO: use a faster HashMap implementation

    // (parser_id, pos) -> Optional<(table_id, entry_id>)
    index: HashMap<(ParserId, usize), (usize, EntryId)>,
    // parser_id -> Optional<table_id>
    table_index: HashMap<ParserId, usize>,
    tables: Vec<ErasedMemoTable<'src>>,
}

impl<'src> Cache<'src> {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            table_index: HashMap::new(),
            tables: Vec::new(),
        }
    }

    fn get_or_create_table<T: 'src>(&mut self, parser_id: ParserId) -> usize {
        self.table_index
            .get(&parser_id)
            .cloned()
            .unwrap_or_else(|| {
                let table_id = self.tables.len();
                self.tables.push(ErasedMemoTable::new::<T>());
                self.table_index.insert(parser_id, table_id);
                table_id
            })
    }

    /// # Safety:
    /// - If there has been at least one set_entry or get_or_create_table call with this parser_id,
    /// then this method must be called with the same T as all those calls.
    ///
    /// This method is only called inside memoized.rs
    pub(crate) unsafe fn get_entry<'a, T: 'src>(
        &'a self,
        parser_id: ParserId,
        pos: usize,
    ) -> Option<&'a T> {
        let (table_id, entry_id) = self.index.get(&(parser_id, pos))?;
        let erased_table = &self.tables[*table_id];
        Some(unsafe { erased_table.get_entry::<T>(*entry_id) })
    }

    /// # Safety:
    /// - If this method or get_or_create_table has ever been called before with the same parser_id,
    /// then the T used in this call must be the same as all those previous calls
    pub(crate) unsafe fn set_entry<'a, T: 'src>(
        &'a mut self,
        parser_id: ParserId,
        pos: usize,
        entry: T,
    ) -> &'a T {
        let table_id = self.get_or_create_table::<T>(parser_id);
        let table = &mut self.tables[table_id];
        let entry_id = unsafe { table.add_entry(entry) };
        self.index.insert((parser_id, pos), (table_id, entry_id));
        unsafe { table.get_entry(entry_id) }
    }
}
