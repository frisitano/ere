//! The SDKs' stand-in for `embedded-alloc` 0.6, which SP1's (`TlsfHeap`) and OpenVM's (`LlffHeap`)
//! runtimes register as their global allocator. The SDK crates patch it in, so each vendor's own
//! allocations come from a bump allocator over the vendor's private region, which the SDK's
//! wrapper around each accelerator rewinds when the call returns ([`Heap::mark`],
//! [`Heap::release`]): nothing a vendor allocates during a call outlives it.
//!
//! The region runs from `_end`, where the vendors' runtimes start their heaps, to `_heap_start`,
//! where the SDK's linker script starts the guest's heap (`_heap_start` to `_heap_end`), so the
//! linker script alone sets its size. The range a vendor passes to [`Heap::init`], which it derives
//! from its own memory layout, is ignored.
//!
//! It frees a block only if it is the most recent one, as temporaries freed in reverse order are,
//! and resizes the most recent block in place. Blocks are 8-byte aligned at least, as the vendors'
//! own allocators give (OpenVM reads its input into a `Vec<u8>` word by word).

#![no_std]

use core::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
    ptr,
};

/// Alignment of every block.
const MIN_ALIGN: usize = 8;

unsafe extern "C" {
    /// End of the program's static data: the first byte of the vendor's region.
    static _end: u8;
    /// First byte of the guest's heap: one past the last byte of the vendor's region.
    static _heap_start: u8;
}

// The linker writes both addresses into the statics' initial values, so code reaches them wherever
// the region is. They are read volatile so the optimizer cannot fold them back into PC-relative
// references, which reach only ±2 GiB.
static mut REGION_START: *const u8 = &raw const _end;
static mut REGION_END: *const u8 = &raw const _heap_start;

/// A bump allocator over the region given to [`Heap::init`].
pub struct Heap {
    next: Cell<usize>,
    end: Cell<usize>,
}

// zkVM guests run on one thread.
unsafe impl Sync for Heap {}

/// `embedded-alloc`'s linked-list heap, as OpenVM names it.
pub type LlffHeap = Heap;
/// `embedded-alloc`'s TLSF heap, as SP1 names it.
pub type TlsfHeap = Heap;

impl Heap {
    /// A heap with no memory, until [`Heap::init`].
    pub const fn empty() -> Self {
        Self {
            next: Cell::new(0),
            end: Cell::new(0),
        }
    }

    /// Gives the heap the vendor's region, `_end` to `_heap_start`. `embedded-alloc`'s
    /// arguments, the range the vendor derives from its own layout, are ignored.
    ///
    /// # Safety
    ///
    /// The heap must not be in use.
    pub unsafe fn init(&self, _start_addr: usize, _size: usize) {
        unsafe {
            self.next
                .set((&raw const REGION_START).read_volatile().addr());
            self.end.set((&raw const REGION_END).read_volatile().addr());
        }
    }

    /// The position to [`Heap::release`] to.
    pub fn mark(&self) -> usize {
        self.next.get()
    }

    /// Frees everything allocated since [`Heap::mark`] returned `mark`.
    ///
    /// # Safety
    ///
    /// Nothing allocated since then may be used again.
    pub unsafe fn release(&self, mark: usize) {
        self.next.set(mark);
    }

    /// The start and end of a block of `size` bytes aligned to `align` (a power of two) at or after
    /// `from`, if it fits before the end of the region. Address arithmetic that would overflow
    /// fails rather than wraps.
    fn fit(&self, from: usize, align: usize, size: usize) -> Option<(usize, usize)> {
        let start = from.checked_add(align - 1)? & !(align - 1);
        let end = start.checked_add(size)?;
        (end <= self.end.get()).then_some((start, end))
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.fit(
            self.next.get(),
            layout.align().max(MIN_ALIGN),
            layout.size(),
        ) {
            Some((start, end)) => {
                self.next.set(end);
                start as *mut u8
            }
            None => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr as usize + layout.size() == self.next.get() {
            self.next.set(ptr as usize);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // The most recent block grows or shrinks in place. `ptr` is already aligned, and the block
        // lies within the region, so `start + layout.size()` cannot overflow.
        let start = ptr as usize;
        if start + layout.size() == self.next.get() {
            if let Some((_, end)) = self.fit(start, 1, new_size) {
                self.next.set(end);
                return ptr;
            }
        }
        let new =
            unsafe { self.alloc(Layout::from_size_align_unchecked(new_size, layout.align())) };
        if !new.is_null() {
            unsafe { ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
        }
        new
    }
}
