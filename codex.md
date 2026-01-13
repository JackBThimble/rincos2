RincOS Architecture Contract

This document defines hard, non-negotiable architectural rules for RincOS.
They exist to preserve correctness, performance, and long-term maintainability in a low-level kernel.

If code violates this document, the code is wrong — not the document.

1. Core design philosophy

RincOS is built around explicit separation of concerns:

Mechanism and policy must never mix

Architecture-specific details must never leak upward

Convenience is never allowed to break invariants

Correctness and analyzability take priority over speed of iteration

This is not about cleanliness — it is about preventing undefined behavior, deadlocks, and un-debuggable systems as complexity increases (SMP, scheduling, IPC).

2. Layer definitions
2.1 arch/* — Architecture mechanism

Purpose

Implements CPU- and platform-specific mechanisms

Allowed

Inline assembly

MSRs / control registers / system registers

Interrupt controllers (APIC, GIC)

Timers (TSC, CNTVCT)

Page tables, TLB, MMU setup

Exception entry/exit

Device register access

Forbidden

Logging

Formatting

Allocation

Scheduling

Policy decisions

Kernel services

References to kernel/*

Rule

arch answers how the hardware works, never what it means.

2.2 kernel/* — Policy and orchestration

Purpose

Owns system behavior and decisions

Allowed

Scheduling policy

IPC

VM policy

Driver orchestration

Logging

Panic handling

Architecture selection (cfg(target_arch))

Forbidden

Inline assembly

Reading/writing registers

CPUID/MSR/system register access

Interrupt controller access

Timer hardware access

Rule

kernel consumes data, never hardware.

2.3 crates/hal — Neutral contracts

Purpose

Defines the data and trait boundaries between arch and kernel

Allowed

POD structs (#[repr(C)])

Traits

Registration hooks

Capability-neutral constants

Forbidden

Inline assembly

cfg(target_arch)

Architecture selection

Logging

Allocation

Policy

Hardware assumptions

Rule

HAL must compile unchanged for architectures that do not yet exist.

2.4 crates/bootabi — Boot ABI

Purpose

Defines boot-time data structures shared with the bootloader

Allowed

Plain data structures

ABI-stable representations

Forbidden

Behavior

Architecture-specific logic

Policy

3. Dependency rules (strict)
Allowed dependency directions
kernel  ───► arch/*
kernel  ───► crates/hal
arch/*  ───► crates/hal
crates/hal ───► crates/bootabi

Forbidden dependencies
arch/*  ─X─► kernel
crates/hal ─X─► kernel
crates/hal ─X─► arch/*


If a change requires breaking this graph, the design is wrong.

4. Assembly and register access
Hard rule

No inline assembly outside arch/*.

This includes:

asm!

rdmsr / wrmsr

mov crX

system registers (ELR_EL1, FAR_EL1, etc.)

CPUID

If the kernel needs information derived from hardware:

arch extracts it

passes it upward as data

5. Interrupt model
Mandatory interrupt flow
stub.S
  → arch dispatch
    → hal::interrupt::dispatch(IrqFrame)
      → kernel InterruptHandler

Responsibilities

arch

Decode vector

Collect architecture-specific state

Perform interrupt acknowledgment (EOI)

Populate IrqFrame

kernel

Interpret meaning (fault, IRQ, timer)

Decide policy (panic, schedule, ignore)

Never touch registers

6. Interrupt frame contract
IrqFrame semantics

Must be #[repr(C)]

Must contain data only

Architecture-specific fields must be abstracted

Example:

x86_64 page fault → CR2 → fault_addr

aarch64 page fault → FAR_EL1 → fault_addr

Kernel code must never know where the value came from.

7. Timers
Architecture layer

arch provides primitive time operations only:

now_ticks()

frequency_hz()

arm_one_shot(deadline_ticks)

No logging. No units. No policy.

Kernel layer

kernel decides:

time units

scheduling deadlines

logging

accounting

Kernel never references TSC, CNTVCT, or timer hardware names.

8. Logging
Logging is policy

Logging does not belong in arch

arch may expose a byte output primitive at most

Formatting, buffering, levels, panic behavior live in kernel

If logging ever appears in arch, it is a violation.

9. Conditional compilation
Allowed

#[cfg(target_arch)] in kernel

#[cfg(target_arch)] in arch/*

Feature flags (#[cfg(feature = "...")]) in kernel or hal

Forbidden

#[cfg(target_arch)] in crates/hal

Any arch selection in hal

10. aarch64 parity requirement

All abstractions crossing arch → hal → kernel must be valid for:

x86_64

aarch64

Even if aarch64 is not implemented yet, designs must not assume:

interrupt vectors are identical

page fault semantics match

timer sources behave the same

If an interface cannot map cleanly to aarch64, it is wrong.

11. “Just this once” rule

There is no “just this once”.

If a rule feels inconvenient:

stop

redesign

preserve invariants

Violating the contract creates long-term debt that always costs more than the redesign.

12. Intent

This document exists to:

prevent entropy

enable SMP safely

make scheduling analyzable

keep performance predictable

allow multi-architecture growth without rewrites

RincOS is intended to be serious systems software.

Final note

If code works but violates this document, it is still wrong.

Correctness first.
Mechanism and policy stay separate.
No soup sandwiches.
