//! Opt-in application allocator instrumentation; committed Wasm capacity is a different metric.
use std::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static BLOCKS: AtomicUsize = AtomicUsize::new(0);
pub struct CountingAllocator<A>(pub A);
fn added(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
// SAFETY: every operation delegates the original pointer/layout to the underlying
// allocator. Atomics neither allocate nor access the memory being allocated.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { self.0.alloc(layout) };
        if !p.is_null() {
            added(layout.size());
            BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        BLOCKS.fetch_sub(1, Ordering::Relaxed);
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { self.0.alloc_zeroed(layout) };
        if !p.is_null() {
            added(layout.size());
            BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { self.0.realloc(p, layout, size) };
        if !next.is_null() {
            if size >= layout.size() {
                added(size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - size, Ordering::Relaxed);
            }
        }
        next
    }
}
pub fn stats() -> (usize, usize, usize) {
    (
        LIVE.load(Ordering::Relaxed),
        PEAK.load(Ordering::Relaxed),
        BLOCKS.load(Ordering::Relaxed),
    )
}
