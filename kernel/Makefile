ARCH ?= x86_64
MODE ?= debug
qemu := qemu-system-$(ARCH)
target := $(ARCH)
build_path := target/$(target)/$(MODE)
ESP := $(build_path)/esp
bootloader := ../bootloader
OVMF := $(bootloader)/OVMF.fd

run: bootloader
	cargo build
	cp $(build_path)/canyon-os $(ESP)/EFI/canyon/kernel.elf
	qemu-system-x86_64 \
	-drive if=pflash,format=raw,readonly,file=$(OVMF) \
	-drive format=raw,file=fat:rw:$(build_path)/esp \
	-m 4G \
	-device isa-debug-exit \
	-net none

bootloader: filesystem $(ESP)/EFI/Boot/boot.conf $(ESP)/EFI/Boot/BootX64.efi

$(ESP)/EFI/Boot/BootX64.efi: $(bootloader)/src/*
	cd $(bootloader); cargo build
	cp $(bootloader)/target/x86_64-unknown-uefi/debug/bootloader.efi $(ESP)/EFI/Boot/BootX64.efi

$(ESP)/EFI/Boot/boot.conf: $(bootloader)/boot.conf
	cp $(bootloader)/boot.conf $(ESP)/EFI/Boot/boot.conf

filesystem:
	mkdir -p $(ESP)/EFI/Boot
	mkdir -p $(ESP)/EFI/canyon

FORCE:
