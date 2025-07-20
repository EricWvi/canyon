#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate alloc;
use alloc::string::String;
use core::any::Any;

pub mod cafs;
pub mod fs;
pub mod vfs;

pub const PARTITION_UUID: &str = "0c421611-8e4a-464e-b683-96265fc14532";

pub const BLOCK_SIZE: u64 = 512;
pub const BLOCK_BITS: u64 = BLOCK_SIZE * 8;

// TODO coverage test

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: u64, buf: &mut [u8]);
    fn write_block(&mut self, block_id: u64, buf: &[u8]);
}

#[derive(Debug)]
pub enum Error {
    NotExist(String),
    RunOutOfInode,
}

pub mod fake {
    use super::{BlockDevice, BLOCK_SIZE};
    use alloc::vec;
    use alloc::vec::Vec;

    #[derive(Debug)]
    pub struct Disk {
        pub total_blocks: u64,
        pub data: Vec<[u8; BLOCK_SIZE as usize]>,
    }

    impl Disk {
        pub fn new(total_blocks: u64) -> Self {
            Self {
                total_blocks,
                data: vec![[0; BLOCK_SIZE as usize]; total_blocks as usize],
            }
        }
    }

    impl BlockDevice for Disk {
        fn read_block(&self, block_id: u64, buf: &mut [u8]) {
            assert!(block_id < self.total_blocks);
            assert_eq!(buf.len(), BLOCK_SIZE as usize);
            buf.copy_from_slice(&self.data[block_id as usize]);
        }

        fn write_block(&mut self, block_id: u64, buf: &[u8]) {
            assert!(block_id < self.total_blocks);
            assert_eq!(buf.len(), BLOCK_SIZE as usize);
            self.data[block_id as usize].copy_from_slice(buf);
        }
    }

    #[test]
    fn test_read_write_block() {
        let mut disk = Disk {
            total_blocks: 5,
            data: vec![[0; BLOCK_SIZE as usize]; 5],
        };
        assert_eq!(disk.data[2], [0; BLOCK_SIZE as usize]);
        let data = [5; BLOCK_SIZE as usize];
        disk.write_block(2, &data[..]);
        let mut out = [0u8; BLOCK_SIZE as usize];
        disk.read_block(2, &mut out);
        assert_eq!(out, data);
    }
}
