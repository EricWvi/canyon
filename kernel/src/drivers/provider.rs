use crate::memory::frame::{alloc_range, dealloc};
use crate::memory::{to_phys_addr, to_virt_addr, PAGE_SIZE};
use isomorphic_drivers::provider;
use log::trace;
use x86_64::structures::paging::PhysFrame;
use x86_64::PhysAddr;

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_dma(size: usize) -> (usize, usize) {
        let paddr = virtio_dma_alloc((size + PAGE_SIZE - 1) / PAGE_SIZE);
        let vaddr = to_virt_addr(paddr.as_u64());
        (vaddr.as_u64() as usize, paddr.as_u64() as usize)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let paddr = to_phys_addr(vaddr as u64);
        virtio_dma_dealloc(paddr, (size + PAGE_SIZE - 1) / PAGE_SIZE);
    }
}

#[no_mangle]
extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let paddr = alloc_range(pages as u64).unwrap();
    let addr = PhysAddr::new(paddr.start_address().as_u64());
    trace!("alloc DMA: paddr={:#x}, pages={}", addr, pages);
    addr
}

#[no_mangle]
extern "C" fn virtio_dma_dealloc(paddr: PhysAddr, pages: usize) {
    let frame = PhysFrame::containing_address(paddr);
    for i in 0..pages as u64 {
        dealloc(frame + i);
    }
    trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
}
