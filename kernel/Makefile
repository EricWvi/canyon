ARCH ?= x86_64
MODE ?= debug
qemu := qemu-system-$(ARCH)
target := $(ARCH)
build_path := target/$(target)/$(MODE)
ESP := $(build_path)/esp
bootloader := ../bootloader/efi
OVMF := $(bootloader)/OVMF.fd

qemu_opts := -drive if=pflash,format=raw,readonly,file=$(OVMF) \
             	-drive format=raw,file=fat:rw:$(build_path)/esp \
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
#!/usr/local/bin/bash

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

run: bootloader kernel
	$(qemu) $(qemu_opts)

test: bootloader
	echo "$$KERNEL_TEST_RUNNER" > target/kernel_test_runner.sh
	cargo test --lib $(build_args)

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

filesystem:
	mkdir -p $(ESP)/EFI/Boot
	mkdir -p $(ESP)/EFI/canyon

lldb:
	lldb $(ESP)/EFI/canyon/kernel.elf

# --gdb-cmd
gdb:
	gdbgui $(ESP)/EFI/canyon/kernel.elf --port 8080 \
		  --gdb-cmd='/Users/wangyi/.cargo/bin/rust-gdb -ex "target remote localhost:1234"'

FORCE:


