use crate::drivers::ahci;
use crate::drivers::ahci::AHCIDriver;
use crate::fs;
use crate::memory::{to_virt_addr, PAGE_SIZE};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use cafs::vfs::VFS;
use cafs::{BlockDevice, BLOCK_SIZE};
use core::str::FromStr;
use gpt_disk_io::gpt_disk_types::BlockSize;
use gpt_disk_io::{Disk, SliceBlockIo};
use log::{debug, info};
use pci::*;
use spin::RwLock;
use uguid::Guid;
use x86_64::instructions::port::Port;

const PCI_COMMAND: u16 = 0x04;
const PCI_CAP_PTR: u16 = 0x34;
const PCI_INTERRUPT_LINE: u16 = 0x3c;
const PCI_INTERRUPT_PIN: u16 = 0x3d;

const PCI_MSI_CTRL_CAP: u16 = 0x00;
const PCI_MSI_ADDR: u16 = 0x04;
const PCI_MSI_UPPER_ADDR: u16 = 0x08;
const PCI_MSI_DATA_32: u16 = 0x08;
const PCI_MSI_DATA_64: u16 = 0x0C;

const PCI_CAP_ID_MSI: u8 = 0x05;

struct PortOpsImpl;

impl PortOps for PortOpsImpl {
    unsafe fn read8(&self, port: u16) -> u8 {
        Port::new(port).read()
    }
    unsafe fn read16(&self, port: u16) -> u16 {
        Port::new(port).read()
    }
    unsafe fn read32(&self, port: u16) -> u32 {
        Port::new(port).read()
    }
    unsafe fn write8(&self, port: u16, val: u8) {
        Port::new(port).write(val);
    }
    unsafe fn write16(&self, port: u16, val: u16) {
        Port::new(port).write(val);
    }
    unsafe fn write32(&self, port: u16, val: u32) {
        Port::new(port).write(val);
    }
}

pub fn init() {
    let pci_iter = unsafe { scan_bus(&PortOpsImpl, CSpaceAccessMethod::IO) };
    for dev in pci_iter {
        init_driver(&dev);
    }
}

pub fn init_driver(dev: &PCIDevice) {
    if dev.id.class == 0x01 && dev.id.subclass == 0x06 {
        // Mass storage class
        // SATA subclass
        if let Some(BAR::Memory(addr, len, _, _)) = dev.bars[5] {
            debug!("Found AHCI dev {:?} BAR5 {:x?}", dev, addr);
            let irq = unsafe { enable(dev.loc) };
            assert!(len as usize <= PAGE_SIZE);
            init_sata(irq, addr, len);
        }
    }
}

/// Enable the pci device and its interrupt
/// Return assigned MSI interrupt number when applicable
unsafe fn enable(loc: Location) -> Option<usize> {
    let ops = &PortOpsImpl;
    let am = CSpaceAccessMethod::IO;

    // 23 and lower are used
    static mut MSI_IRQ: u32 = 23;

    let orig = am.read16(ops, loc, PCI_COMMAND);
    // IO Space | MEM Space | Bus Mastering | Special Cycles | PCI Interrupt Disable
    am.write32(ops, loc, PCI_COMMAND, (orig | 0x40f) as u32);

    // find MSI cap
    let mut msi_found = false;
    let mut cap_ptr = am.read8(ops, loc, PCI_CAP_PTR) as u16;
    let mut assigned_irq = None;
    while cap_ptr > 0 {
        let cap_id = am.read8(ops, loc, cap_ptr);
        if cap_id == PCI_CAP_ID_MSI {
            let orig_ctrl = am.read32(ops, loc, cap_ptr + PCI_MSI_CTRL_CAP);
            // The manual Volume 3 Chapter 10.11 Message Signalled Interrupts
            // 0 is (usually) the apic id of the bsp.
            am.write32(ops, loc, cap_ptr + PCI_MSI_ADDR, 0xfee00000 | (0 << 12));
            MSI_IRQ += 1;
            let irq = MSI_IRQ;
            assigned_irq = Some(irq as usize);
            // we offset all our irq numbers by 32
            if (orig_ctrl >> 16) & (1 << 7) != 0 {
                // 64bit
                am.write32(ops, loc, cap_ptr + PCI_MSI_DATA_64, irq + 32);
            } else {
                // 32bit
                am.write32(ops, loc, cap_ptr + PCI_MSI_DATA_32, irq + 32);
            }

            // enable MSI interrupt, assuming 64bit for now
            am.write32(ops, loc, cap_ptr + PCI_MSI_CTRL_CAP, orig_ctrl | 0x10000);
            debug!(
                "MSI control {:#b}, enabling MSI interrupt {}",
                orig_ctrl >> 16,
                irq
            );
            msi_found = true;
        }
        debug!("PCI device has cap id {} at {:#X}", cap_id, cap_ptr);
        cap_ptr = am.read8(ops, loc, cap_ptr + 1) as u16;
    }

    if !msi_found {
        // Use PCI legacy interrupt instead
        // IO Space | MEM Space | Bus Mastering | Special Cycles
        am.write32(ops, loc, PCI_COMMAND, (orig | 0xf) as u32);
        debug!("MSI not found, using PCI interrupt");
    }

    info!("pci device enable done");

    assigned_irq
}

#[derive(Debug)]
struct Partition {
    pub starting_lba: u64,
    pub ending_lba: u64,
    pub name: String,
    pub guid: Guid,
}

struct BLK {
    offset: u64,
    driver: Arc<AHCIDriver>,
}

impl BlockDevice for BLK {
    fn read_block(&self, block_id: u64, buf: &mut [u8]) {
        assert_eq!(buf.len(), BLOCK_SIZE as usize);
        self.driver.read_block(block_id + self.offset, buf);
    }

    fn write_block(&mut self, block_id: u64, buf: &[u8]) {
        assert_eq!(buf.len(), BLOCK_SIZE as usize);
        self.driver.write_block(block_id + self.offset, buf);
    }
}

fn init_sata(irq: Option<usize>, bar_addr: u64, bar_len: u32) {
    let vaddr = to_virt_addr(bar_addr);
    if let Some(driver) = ahci::init(irq, vaddr.as_u64() as usize, bar_len as usize) {
        let mut gpt_data = Vec::with_capacity(2048 * 512);
        for i in 0..2048 {
            let mut buf = [0; 512];
            driver.read_block(i, &mut buf[..]);
            gpt_data.extend_from_slice(&buf);
        }
        let bs = BlockSize::BS_512;
        let block_io = SliceBlockIo::new(&gpt_data[..], bs);
        let mut disk = Disk::new(block_io).unwrap();

        let mut header_data = vec![0u8; 512];
        let header = disk.read_primary_gpt_header(&mut header_data[..]).unwrap();
        let layout = header.get_partition_entry_array_layout().unwrap();
        let mut layout_data = vec![0u8; layout.num_bytes_rounded_to_block(bs).unwrap() as usize];
        let partitions_array = disk
            .read_gpt_partition_entry_array(layout, &mut layout_data[..])
            .unwrap();
        let mut partitions = vec![];

        for i in 0..layout.num_entries {
            let p = partitions_array.get_partition_entry(i).unwrap();
            let type_guid = p.partition_type_guid;
            if type_guid.0 != Guid::from_str(cafs::PARTITION_UUID).unwrap() {
                continue;
            }
            partitions.push(Partition {
                starting_lba: p.starting_lba.clone().to_u64(),
                ending_lba: p.ending_lba.clone().to_u64(),
                name: p.name.to_string(),
                guid: p.partition_type_guid.0,
            });
        }
        let blk = BLK {
            offset: partitions[0].starting_lba,
            driver,
        };
        let cafs = VFS::new(Arc::new(RwLock::new(blk)));
        unsafe {
            fs::VFS = Some(cafs);
        }
        // info!("{:?}", cafs.ls_root());
        // let contents = cafs.read_unstable("/hello").unwrap();
        // info!("hello len: {}", contents.len());
    }
}
