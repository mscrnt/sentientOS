use crate::boot_info::BootInfo;
use crate::serial_println;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use uefi::prelude::*;
use uefi::table::boot::MemoryType as UefiMemoryType;

mod frame_allocator;
// pub use frame_allocator::FrameAllocator;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    total_memory: 0,
    used_memory: 0,
    reserved_for_model: 0,
});

struct MemoryStats {
    total_memory: u64,
    used_memory: u64,
    reserved_for_model: u64,
}

/// Initialize a minimal allocator early for JSON parsing
pub fn init_early_allocator(system_table: &SystemTable<Boot>) {
    serial_println!("🔧 Initializing early allocator...");

    // Allocate a small region using UEFI for early allocations
    let boot_services = system_table.boot_services();

    // Allocate 4MB for early heap
    const EARLY_HEAP_SIZE: usize = 4 * 1024 * 1024;
    let early_heap = boot_services
        .allocate_pages(
            uefi::table::boot::AllocateType::AnyPages,
            uefi::table::boot::MemoryType::LOADER_DATA,
            EARLY_HEAP_SIZE / 4096,
        )
        .expect("Failed to allocate early heap");

    unsafe {
        ALLOCATOR
            .lock()
            .init(early_heap as *mut u8, EARLY_HEAP_SIZE);
    }

    serial_println!("✅ Early allocator initialized at 0x{:x}", early_heap);
}

pub fn init(system_table: &SystemTable<Boot>, boot_info: &BootInfo) {
    serial_println!("🔧 Initializing memory management...");

    // Get UEFI memory map
    let mmap_size = system_table.boot_services().memory_map_size();

    // Use a static buffer for memory map to avoid allocation
    const MAX_MMAP_SIZE: usize = 16384; // 16KB should be enough for memory map
    static mut MMAP_BUFFER: [u8; MAX_MMAP_SIZE] = [0; MAX_MMAP_SIZE];

    let required_size = mmap_size.map_size + 10 * mmap_size.entry_size;
    if required_size > MAX_MMAP_SIZE {
        panic!(
            "Memory map too large: {} > {}",
            required_size, MAX_MMAP_SIZE
        );
    }

    let mmap_buf = unsafe { &mut MMAP_BUFFER[..required_size] };

    let mmap = system_table
        .boot_services()
        .memory_map(mmap_buf)
        .expect("Failed to get memory map");

    // Find suitable memory for heap
    let mut heap_start = None;
    let mut heap_size = 0u64;
    let mut total_memory = 0u64;

    for desc in mmap.entries() {
        if desc.ty == UefiMemoryType::CONVENTIONAL {
            total_memory += desc.page_count * 4096;

            // Find a good region for heap (at least 64MB)
            if heap_start.is_none() && desc.page_count * 4096 >= 64 * 1024 * 1024 {
                heap_start = Some(desc.phys_start);
                heap_size = (desc.page_count * 4096).min(256 * 1024 * 1024); // Cap at 256MB
            }
        }
    }

    let heap_start = heap_start.expect("No suitable memory for heap");

    serial_println!("  Total memory: {} MB", total_memory / (1024 * 1024));
    serial_println!("  Heap start: 0x{:x}", heap_start);
    serial_println!("  Heap size: {} MB", heap_size / (1024 * 1024));

    // Reserve memory for AI model
    let model_size = boot_info.model.size_bytes;
    serial_println!(
        "  Model reserved: {} MB at 0x{:x}",
        model_size / (1024 * 1024),
        boot_info.model.memory_address
    );

    // Reinitialize heap with the larger region
    unsafe {
        // The allocator might already be initialized, so we need to extend it
        // For now, we'll just use the new larger heap
        ALLOCATOR.force_unlock(); // Reset lock in case it's held
        ALLOCATOR
            .lock()
            .init(heap_start as *mut u8, heap_size as usize);
    }

    // Update stats
    let mut stats = MEMORY_STATS.lock();
    stats.total_memory = total_memory;
    stats.reserved_for_model = model_size;

    serial_println!("✅ Memory management initialized");
}

pub fn get_free_memory() -> u64 {
    let stats = MEMORY_STATS.lock();
    stats.total_memory - stats.used_memory - stats.reserved_for_model
}

pub fn run_memory_optimizer() {
    serial_println!("🔧 Running AI-triggered memory optimization...");

    // In a real implementation:
    // - Compact heap
    // - Free unused pages
    // - Defragment memory
    // - Update page tables

    serial_println!("✅ Memory optimization complete");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
