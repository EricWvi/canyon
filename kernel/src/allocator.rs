use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use spin::Mutex;

#[alloc_error_handler]
fn out_of_memory(layout: Layout) -> ! {
    panic!(
        "Ran out of free memory while trying to allocate {:#?}",
        layout
    )
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator {
    start: Mutex::new(Cell::new(0xffff_9000_4000_0000)),
};

struct Allocator {
    start: Mutex<Cell<usize>>,
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let v = self.start.lock();
        let start = v.get() + layout.align() - v.get() % layout.align();
        v.set(start + layout.size());
        start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
