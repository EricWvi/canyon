ARCH ?= x86_64
MODE ?= debug
qemu := qemu-system-$(ARCH)
target := $(ARCH)
build_path := target/$(target)/$(MODE)
ESP := $(build_path)/esp
OVMF := OVMF.fd

# -serial mon:stdio -nographic
run: FORCE
	cargo build
	mkdir -p target/x86_64-unknown-uefi/debug/esp/EFI/Boot
	mkdir -p target/x86_64-unknown-uefi/debug/esp/EFI/canyon
	cp target/x86_64-unknown-uefi/debug/bootloader.efi target/x86_64-unknown-uefi/debug/esp/EFI/Boot/BootX64.efi
	cp ../../kernel/target/x86_64/debug/canyon-os target/x86_64-unknown-uefi/debug/esp/EFI/canyon/kernel.elf
	cp boot.conf target/x86_64-unknown-uefi/debug/esp/EFI/Boot/boot.conf
	qemu-system-x86_64 \
	-drive if=pflash,format=raw,readonly,file=OVMF.fd \
	-drive format=raw,file=fat:rw:target/x86_64-unknown-uefi/debug/esp \
	-m 4G \
	-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
	-serial mon:stdio -nographic \
	-net none

FORCE:
