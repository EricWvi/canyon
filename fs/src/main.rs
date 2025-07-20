use cafs::cafs::CAFS;
use cafs::fake::Disk;
use cafs::fs::FS;
use cafs::BLOCK_SIZE;
use spin::RwLock;
use std::ptr::slice_from_raw_parts;
use std::sync::Arc;
use std::{env, fs, process};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        process::exit(64);
    } else if args.len() == 2 {
        create_img(args[1].parse().expect("Wanted a number"))?;
    } else {
        create_img(50)?;
    }
    Ok(())
}

fn create_img(size: usize) -> std::io::Result<()> {
    let total_blocks = (2 * size as u64) << 10;
    let inode_bitmap_blocks = 10;
    let disk = Disk::new(total_blocks);
    let data = slice_from_raw_parts(
        disk.data.as_ptr() as *const u8,
        disk.data.len() * BLOCK_SIZE as usize,
    );
    let fs = CAFS::init(
        Arc::new(RwLock::new(disk)),
        total_blocks,
        inode_bitmap_blocks,
    );
    let inode = fs.create(0, "test.txt".to_string());
    let inode_number = inode.read().inode_number();
    fs.write(inode_number, &Vec::from("Test File".as_bytes()));

    let file = fs::read("rootfs/hello").unwrap();
    let inode = fs.create(0, "hello".to_string());
    let inode_number = inode.read().inode_number();
    fs.write(inode_number, &file);

    fs.flush();

    unsafe {
        fs::write("cafs.bin", &*data)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use cafs::fake::Disk;
    use cafs::vfs::VFS;
    use cafs::BLOCK_SIZE;
    use spin::RwLock;
    use std::fs;
    use std::sync::Arc;

    use log::{Level, Metadata, Record};

    struct SimpleLogger;

    impl log::Log for SimpleLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                println!("{} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }

    #[test]
    fn test_cafs() {
        static LOGGER: SimpleLogger = SimpleLogger;
        log::set_logger(&LOGGER).map(|()| log::set_max_level(log::STATIC_MAX_LEVEL));

        let data = fs::read("cafs.bin").unwrap();
        let vec_u8_ptr = data.as_ptr();
        let num_chunks = data.len() / BLOCK_SIZE as usize;
        let vec_u8_array: Vec<[u8; BLOCK_SIZE as usize]> = (0..num_chunks)
            .map(|i| unsafe {
                let ptr = vec_u8_ptr.add(i * BLOCK_SIZE as usize);
                std::ptr::read(ptr as *const [u8; BLOCK_SIZE as usize])
            })
            .collect();
        let disk = Disk {
            total_blocks: vec_u8_array.len() as u64,
            data: vec_u8_array,
        };
        let cafs = VFS::new(Arc::new(RwLock::new(disk)));
        println!("{:?}", cafs.ls_root());
        let contents = cafs.read_unstable("/test.txt").unwrap();
        let str = String::from_utf8_lossy(&contents).to_string();
        println!("test.txt: {}", str);
    }
}
