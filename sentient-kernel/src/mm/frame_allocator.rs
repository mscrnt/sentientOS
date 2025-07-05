use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::PhysAddr;

#[allow(dead_code)]
pub struct FrameAllocator {
    next_free: PhysAddr,
    memory_end: PhysAddr,
}

impl FrameAllocator {
    #[allow(dead_code)]
    pub fn new(start: PhysAddr, end: PhysAddr) -> Self {
        FrameAllocator {
            next_free: start,
            memory_end: end,
        }
    }

    #[allow(dead_code)]
    pub fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.next_free < self.memory_end {
            let frame = PhysFrame::containing_address(self.next_free);
            self.next_free += 4096u64;
            Some(frame)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn deallocate_frame(&mut self, _frame: PhysFrame<Size4KiB>) {
        // Simple allocator doesn't support deallocation
        // In a real implementation, maintain a free list
    }
}
