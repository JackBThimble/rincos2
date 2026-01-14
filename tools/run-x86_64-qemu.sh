#!/usr/bin/env bash
set -e

OVMF_CODE=ovmf.fd
OVMF_VARS=ovmf_vars.fd

qemu-system-x86_64 \
  -cpu host,+invtsc \
  -enable-kvm \
  -m 1G \
  -smp 1 \
  -drive if=pflash,format=raw,readonly=on,file=$OVMF_CODE \
  -drive if=pflash,format=raw,file=$OVMF_VARS \
  -cdrom rincos.iso \
  -serial stdio \
  -no-shutdown \
  -no-reboot \
  -d guest_errors,cpu_reset,int

