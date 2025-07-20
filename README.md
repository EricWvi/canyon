VFS 提供了两个针对文件系统对象的缓存 INode Cache 和 DEntry Cache，它们缓存最近使用过的文件系统对象，用来加快对 INode 和 DEntry 的访问。Linux 内核还提供了 Buffer Cache 缓冲区，用来缓存文件系统和相关块设备之间的请求，减少访问物理设备的次数，加快访问速度。Buffer Cache 以 LRU 列表的形式管理缓冲区。

`sudo apt install qemu-system-x86`
`sudo apt install build-essential`
`rustup component add llvm-tools-preview`
`cargo install cargo-binutils`
`touch kernel/target/test-func`

