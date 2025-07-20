ARCH ?= x86_64
MODE ?= debug
qemu := qemu-system-$(ARCH)
target := $(ARCH)
build_path := target/$(target)/$(MODE)
ESP := $(build_path)/esp
bootloader := ../bootloader/efi
fs_code := ../fs
OVMF := $(bootloader)/OVMF.fd

qemu_opts := -drive if=pflash,format=raw,readonly,file=$(OVMF) \
             	-drive format=raw,file=fat:rw:$(build_path)/esp \
             	-drive format=raw,file=$(build_path)/disk.img,id=disk,if=none \
             	-device ahci,id=ahci0 \
                -device ide-hd,drive=disk,bus=ahci0.0 \
             	-m 4G \
             	-device isa-debug-exit,iobase=0xf4,iosize=0x04

# gdb-remote localhost:1234
ifeq ($(GDB), on)
qemu_opts += -s -S
endif

ifeq ($(Serial), on)
qemu_opts += -serial mon:stdio
endif

ifeq ($(Graphic), off)
qemu_opts += -nographic
endif

# -monitor tcp:127.0.0.1:4444

define KERNEL_TEST_RUNNER
#!/bin/bash

cp "$$1" target/x86_64/debug/esp/EFI/canyon/kernel.elf
$(qemu) $(qemu_opts)
exit_code="$$?"
if [[ $$exit_code == 33 ]]; then
  exit 0
else
  exit $$exit_code
fi

endef

export KERNEL_TEST_RUNNER

FEATURES = qemu

build_args := --features "$(FEATURES)" --target targets/$(ARCH).json
test_build_args := --lib $(Test) --features "$(FEATURES)" --target targets/$(ARCH).json

run: bootloader kernel
	$(qemu) $(qemu_opts)

test: bootloader
	echo "$$Test" > target/test-func
	echo "$$KERNEL_TEST_RUNNER" > target/kernel_test_runner.sh
	chmod +x target/kernel_test_runner.sh
	cargo test $(test_build_args)

kernel: filesystem $(ESP)/EFI/canyon/kernel.elf

bootloader: filesystem $(ESP)/EFI/Boot/boot.conf $(ESP)/EFI/Boot/BootX64.efi

$(ESP)/EFI/canyon/kernel.elf: $(build_path)/canyon-os
	cp $< $@

$(build_path)/canyon-os: $(shell find src -type f)
	cargo build $(build_args)
	rust-objcopy --only-keep-debug $@ $@.sym
	rust-objcopy --strip-debug $@

$(ESP)/EFI/Boot/BootX64.efi: $(shell find ../bootloader/src -type f)
	cd $(bootloader); cargo build
	cp $(bootloader)/target/x86_64-unknown-uefi/debug/bootloader.efi $@

$(ESP)/EFI/Boot/boot.conf: $(bootloader)/boot.conf
	cp $< $@

define PART_DISK
#!/bin/bash

gdisk disk.img <<EOF
n


50M
0700
n



0c421611-8e4a-464e-b683-96265fc14532
c
1
boot
c
2
cafs
w
Y
EOF

endef
export PART_DISK

filesystem: $(build_path)/disk.img
	mkdir -p $(ESP)/EFI/Boot
	mkdir -p $(ESP)/EFI/canyon

$(build_path)/disk.img:
	mkdir -p $(build_path)
	cd $(fs_code); cargo run
	dd if=/dev/zero of=$(build_path)/disk.img bs=1M count=120
	cd $(build_path); bash -c "$$PART_DISK"
	dd if=$(fs_code)/cafs.bin of=$(build_path)/disk.img bs=512 seek=104448 conv=notrunc

lldb:
	lldb $(ESP)/EFI/canyon/kernel.elf

# --gdb-cmd
gdb:
	gdbgui $(ESP)/EFI/canyon/kernel.elf --port 8080 \
		  --gdb-cmd='/Users/wangyi/.cargo/bin/rust-gdb -ex "target remote localhost:1234"'

FORCE:


