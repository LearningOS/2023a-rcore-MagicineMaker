//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.
mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry};
pub use page_table::{PTEFlags, PageTable};
/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

/// into mem_area to current task's mem_set 
pub fn current_insert_area(start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
    use crate::task::current_task;
    let tcb = current_task().unwrap();
    let mut tcb_inner = tcb.inner_exclusive_access();
    tcb_inner.memory_set.insert_framed_area(start_va, end_va, permission);
}

/// shrink mem_area of current task's mem_set
/// start_va: must be an area's start
/// end_va: will be this area's new_start 
pub fn current_remove_area(start_va: VirtAddr, end_va: VirtAddr){
    use crate::task::current_task;
    let tcb = current_task().unwrap();
    let mut tcb_inner = tcb.inner_exclusive_access();
    tcb_inner.memory_set.remove(start_va, end_va);
}