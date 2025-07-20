use crate::drivers::provider::Provider;
use alloc::sync::Arc;
use isomorphic_drivers::block::ahci::AHCI;
use spin::Mutex;

pub struct AHCIDriver(Mutex<AHCI<Provider>>);

impl AHCIDriver {
    pub fn read_block(&self, block_id: u64, buf: &mut [u8]) {
        let mut driver = self.0.lock();
        driver.read_block(block_id as usize, buf);
    }

    pub fn write_block(&self, block_id: u64, buf: &[u8]) {
        let mut driver = self.0.lock();
        driver.write_block(block_id as usize, buf);
    }
}

pub fn init(_irq: Option<usize>, header: usize, size: usize) -> Option<Arc<AHCIDriver>> {
    if let Some(ahci) = AHCI::new(header, size) {
        let driver = Arc::new(AHCIDriver(Mutex::new(ahci)));
        Some(driver)
    } else {
        None
    }
}
