# bootloader
bootloader for Canyon OS

`uefi-rs` provides an allocator, a logger, and a panic handler.

UEFI is already under long mode，so we need not to jump to long mode.

### Load Kernel

Load kernel file from the FAT file system at address allocated by BootServices.

### Page Table

Use BootServices to retrieve the current memory map and compute the max physical address.

During BootServices, UEFI and bootloader runs identity mapped.

### Map Virtual Memory

Read and parse the kernel in ELF format. 🔗 [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format)

Map segment's virtual address(page) to actual physical address(frame).

Map kernel stack and physical memory as per config file.

### Jump

Collect BootInfo and jump to kernel's entry.

```rust
#[repr(C)]
pub struct BootInfo {
    pub memory_map: Vec<&'static MemoryDescriptor>,
    /// The offset where the physical memory is mapped at in the virtual address space.
    pub physical_memory_offset: u64,
    /// The graphic output information
    pub graphic_info: GraphicInfo,
}
```


### Graphics



