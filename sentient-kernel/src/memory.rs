use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, 
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use crate::boot_info::{BootInfo, MemoryRegion, MemoryType};
use log::info;

pub struct BootInfoFrameAllocator {
    memory_map: &'static [MemoryRegion],
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static [MemoryRegion]) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| matches!(r.region_type, MemoryType::Conventional));
        
        let addr_ranges = usable_regions.map(|r| r.start..r.start + r.size);
        
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub fn init(boot_info: &BootInfo) {
    info!("🔧 Initializing memory management...");
    
    // Log memory regions
    let total_memory: u64 = boot_info.memory_map.iter()
        .filter(|r| matches!(r.region_type, MemoryType::Conventional))
        .map(|r| r.size)
        .sum();
    
    info!("  Total usable memory: {} MB", total_memory / (1024 * 1024));
    
    // Log model memory reservation
    if let Some(model_addr) = boot_info.model_physical_address() {
        info!("  AI Model reserved at: 0x{:016x} ({} MB)", 
            model_addr, 
            boot_info.model.size_bytes / (1024 * 1024)
        );
    }
}

/// Create a new OffsetPageTable
pub unsafe fn init_offset_page_table(
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}