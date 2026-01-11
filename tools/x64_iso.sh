#!/bin/sh
set -e

mkdir -p iso/boot
mkdir -p iso/EFI/BOOT

cp target/x86_64-unknown-none/debug/kernel iso/boot/rincos
cp limine/bin/limine-bios.sys iso/boot/
cp limine/bin/limine-bios-cd.bin iso/boot/
cp limine/bin/limine-uefi-cd.bin iso/boot/
cp limine.conf iso/boot

cp limine/bin/BOOTX64.EFI iso/EFI/BOOT

xorriso -as mkisofs \
  -R -r -J \
  -b boot/limine-bios-cd.bin \
  -no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus \
  -apm-block-size 2048 --efi-boot boot/limine-uefi-cd.bin \
  -efi-boot-part --efi-boot-image --protective-msdos-label \
  iso -o rincos.iso

./limine/bin/limine bios-install rincos.iso

