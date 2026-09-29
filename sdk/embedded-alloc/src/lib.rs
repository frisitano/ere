//! The SDKs' stand-in for `embedded-alloc` 0.6, which SP1's (`TlsfHeap`) and OpenVM's (`LlffHeap`)
//! runtimes register as their global allocator. The SDK crates patch it in, so each vendor's own
//! allocations come from a bump allocator over the vendor's private region, which the SDK's
//! wrapper around each accelerator rewinds when the call returns ([`Heap::mark`],
//! [`Heap::release`]): nothing a vendor allocates during a call outlives it.
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
        Self { next: Cell::new(0), end: Cell::new(0) }
    }

    /// Gives the heap `size` bytes from `start_addr`.
    ///
    /// # Safety
    ///
    /// The region must be valid, unused memory, and the heap must not be in use.
    pub unsafe fn init(&self, start_addr: usize, size: usize) {
        self.next.set(start_addr);
        self.end.set(start_addr + size);
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
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(MIN_ALIGN);
        let start = (self.next.get() + align - 1) & !(align - 1);
        match start.checked_add(layout.size()) {
            Some(end) if end <= self.end.get() => {
                self.next.set(end);
                start as *mut u8
            }
            _ => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr as usize + layout.size() == self.next.get() {
            self.next.set(ptr as usize);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let start = ptr as usize;
        if start + layout.size() == self.next.get() && start + new_size <= self.end.get() {
            self.next.set(start + new_size);
            return ptr;
        }
        let new = unsafe { self.alloc(Layout::from_size_align_unchecked(new_size, layout.align())) };
        if !new.is_null() {
            unsafe { ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
        }
        new
    }
}
