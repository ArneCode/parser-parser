//! Parse-scoped memoization storage for [`crate::parser::memoized::Memoized`].
//!
//! # Problem
//!
//! [`MemoStore`] must hold **one [`HashMap`] per memo parser id**, but each [`Memoized`](crate::parser::memoized::Memoized)
//! wrapper has a **different** output type `POut`. Those maps live in one [`HashMap`] keyed by id, so we need a
//! **heterogeneous** map at the crate level while each **row** is homogeneous.
//!
//! Parser outputs may **borrow the parse input** (`POut: 'src`), so we **cannot** use `std::any::Any` /
//! `std::any::TypeId` for the table bodies: `Any` requires the erased concrete type to be `'static`,
//! which arbitrary user `POut` does not satisfy.
//!
//! # Solution (thin pointer + typed drop)
//!
//! Each per-id table is stored as:
//!
//! - a **thin** `NonNull<()>` (type-erased address only), plus
//! - a **typed** [`drop`](ErasedMemoTable::drop_fn) function pointer that knows how to drop the real
//!   `Box<HashMap<usize, MemoEntry<T>>>`.
//!
//! Access is **never** by runtime type id: only [`MemoStore::get_entry`] / [`MemoStore::table_mut`], which are
//! generic in `T` and tie `T` to the call site (see [`crate::parser::memoized::Memoized`]).
//!
//! # Safety contract (why this is sound)
//!
//! 1. **Per-id type discipline**  
//!    For a fixed [`MemoParserId`], the allocation is **always** a `HashMap<usize, MemoEntry<T>>` for the **same**
//!    `T` as when that row was [`ErasedMemoTable::new_for`].  
//!    The only creators are [`MemoStore::table_mut::<T>`] and [`ErasedMemoTable::new_for::<T>`]; the only readers are
//!    [`MemoStore::get_entry::<T>`] / [`table_mut`](MemoStore::table_mut) with the **same** `T`.  
//!    In practice every call comes from [`crate::parser::memoized::Memoized::<P>::parse`](crate::parser::memoized::Memoized),
//!    where `T = POut` is fixed per wrapper and [`Memoized::id`](crate::parser::memoized::Memoized::id) is unique per
//!    `Memoized::new` allocation, so **different wrappers do not share an id**.
//!
//! 2. **Parse lifetime**  
//!    [`MemoStore<'src>`](MemoStore) is only stored in [`crate::context::ParserContext::memo_store`], which is
//!    **`ParserContext<'src>`**. The context is created for one parse and dropped before the input of lifetime `'src`
//!    can be invalidated, so **no memo table outlives** the data its `Rc<POut>` may borrow.
//!
//! 3. **Thin pointer erasure**  
//!    We convert `NonNull<HashMap<usize, MemoEntry<T>>>` → `NonNull<()>` with [`NonNull::cast`] because both
//!    are a **single non-null pointer** with identical layout. We are **not** claiming the *heap payload* is
//!    `'static`; we only erase the **static** type of the **stored pointer** so different `T` can live in one map.
//!    Every dereference or drop goes through the matching `T` from the generic API above.
//!
//! 4. **`commit_on` / recovery**  
//!    Swapping [`MemoStore`](crate::context::ParserContext::memo_store) moves owned tables; it does not change the
//!    per-id typing contract as long as swaps are paired (see [`crate::matcher::commit_matcher`]).
//!
//! # Unsafe surface
//!
//! All `unsafe` in this module implements the contract above: [`drop_erased_hashmap`],
//! and the pointer reads in [`ErasedMemoTable::as_ref`] / [`as_mut`](ErasedMemoTable::as_mut).

use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

/// Opaque id assigned by [`crate::parser::memoized::Memoized`].
pub type MemoParserId = usize;

/// One memo cell: `None` means cached miss; `Some` stores shared output and end byte offset.
pub type MemoEntry<T> = Option<(Rc<T>, usize)>;

/// Convert a typed memo-table pointer to an erased thin pointer.
///
/// Same address as `ptr`; only the static pointee type changes for storage. The **safety contract** is unchanged:
/// see [module-level documentation](self#safety-contract-why-this-is-sound). Callers must use only
/// [`drop_erased_hashmap::<T>`] and [`ErasedMemoTable::as_ref`] / [`as_mut`](ErasedMemoTable::as_mut) with this **same** `T`.
#[inline]
fn erase_hashmap_ptr<'src, T: 'src>(
    ptr: NonNull<HashMap<usize, MemoEntry<T>>>,
) -> NonNull<()> {
    // Not `Box<dyn Any>`: that would be a fat pointer. Here we only narrow the pointer type (`cast`); dereference/drop stay typed.
    ptr.cast()
}

unsafe fn drop_erased_hashmap<'src, T: 'src>(ptr: NonNull<()>) {
    unsafe {
        drop(Box::from_raw(
            ptr.as_ptr().cast::<HashMap<usize, MemoEntry<T>>>(),
        ));
    }
}

struct ErasedMemoTable<'src> {
    ptr: NonNull<()>,
    drop_fn: unsafe fn(NonNull<()>),
    _marker: PhantomData<&'src mut ()>,
}

impl<'src> Drop for ErasedMemoTable<'src> {
    fn drop(&mut self) {
        // SAFETY: `drop_fn` was installed together with `ptr` in `new_for::<T>` for the same `T`.
        unsafe { (self.drop_fn)(self.ptr) };
    }
}

impl<'src> ErasedMemoTable<'src> {
    fn new_for<T: 'src>() -> Self {
        let raw = Box::into_raw(Box::new(HashMap::<usize, MemoEntry<T>>::new()));
        let typed = NonNull::new(raw).expect("non-null from Box::into_raw");
        let ptr = erase_hashmap_ptr(typed);
        Self {
            ptr,
            drop_fn: drop_erased_hashmap::<T>,
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// `T` must match the type used when this table was created with [`Self::new_for`].
    unsafe fn as_ref<T: 'src>(&self) -> &HashMap<usize, MemoEntry<T>> {
        // SAFETY: same `T` as at creation; see module safety contract.
        unsafe { &*self.ptr.as_ptr().cast::<HashMap<usize, MemoEntry<T>>>() }
    }

    /// # Safety
    /// `T` must match the type used when this table was created with [`Self::new_for`].
    unsafe fn as_mut<T: 'src>(&mut self) -> &mut HashMap<usize, MemoEntry<T>> {
        // SAFETY: same `T` as at creation; see module safety contract.
        unsafe { &mut *self.ptr.as_ptr().cast::<HashMap<usize, MemoEntry<T>>>() }
    }
}

/// Heterogeneous memo tables keyed by parser id; values are tied to parse lifetime `'src`.
#[derive(Default)]
pub struct MemoStore<'src> {
    tables: HashMap<MemoParserId, ErasedMemoTable<'src>>,
}

impl<'src> MemoStore<'src> {
    /// Look up a memo cell without creating a table. Returns `None` if there is no entry yet.
    pub fn get_entry<T: 'src>(&self, parser_id: MemoParserId, pos: usize) -> Option<MemoEntry<T>> {
        let erased = self.tables.get(&parser_id)?;
        unsafe { erased.as_ref::<T>() }.get(&pos).cloned()
    }

    /// Returns the typed memo table for `parser_id`, creating it on first use.
    ///
    /// # Type invariant
    ///
    /// Callers must always use the same `T` for a given `parser_id` (enforced by [`crate::parser::memoized::Memoized`];
    /// see [module safety contract](self#safety-contract-why-this-is-sound)).
    pub fn table_mut<T: 'src>(&mut self, parser_id: MemoParserId) -> &mut HashMap<usize, MemoEntry<T>> {
        if !self.tables.contains_key(&parser_id) {
            self.tables.insert(parser_id, ErasedMemoTable::new_for::<T>());
        }
        unsafe {
            self.tables
                .get_mut(&parser_id)
                .expect("memo table just inserted")
                .as_mut::<T>()
        }
    }
}
