.intel_syntax noprefix
.section .text

.extern exception_dispatch
.extern irq_dispatch

// Export tables of stub entry addresses
.global isr_stub_table
.global irq_stub_table

// Which exceptions push an error code automatically?
// 8,10,11,12,13,14,17
.macro PUSH_GPRS
    push rax
    push rbx
    push rcx
    push rdx
    push rbp
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
.endm

.macro POP_GPRS
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rbp
    pop rdx
    pop rcx
    pop rbx
    pop rax
.endm

// Common handler for exceptions: stack has (top -> bottom):
//  [gprs...]
//  vector
//  error_code
//  CPU frame: rip, cs, rflags, rsp, ss
//
// We pass &ExceptionContext in RDI (SysV ABI)
.macro EXC_COMMON
    PUSH_GPRS

    // RDI = &ctx (current RSP)
    mov rdi, rsp
    call exception_dispatch

    POP_GPRS
    add rsp, 16          // pop vector + error_code
    iretq
.endm

.macro IRQ_COMMON
    PUSH_GPRS
    mov rdi, rsp
    call irq_dispatch
    POP_GPRS
    add rsp, 16
    iretq
.endm

// Stub for exception WITHOUT error code: push 0, push vec
.macro ISR_NOERR vec
.global isr_\vec
isr_\vec:
    push 0
    push \vec
    EXC_COMMON
.endm

// Stub for exception WITH error code: CPU already pushed errcode, so we only push vec.
// But we want uniform layout (vector, error_code) above CPU frame.
// CPU stack: errcode, rip, cs, rflags, rsp, ss
// We need: vector, errcode, rip...
.macro ISR_ERR vec
.global isr_\vec
isr_\vec:
    // stack top is error_code already
    push \vec
    // swap so layout becomes: vector, error_code, ...
    mov rax, [rsp]
    xchg rax, [rsp+8]
    mov [rsp], rax

    EXC_COMMON
.endm

// IRQ stubs: no error code; push 0, push vector
.macro IRQ vec
.global irq_\vec
irq_\vec:
    push 0
    push \vec
    IRQ_COMMON
.endm

// Define exception stubs 0..31
ISR_NOERR 0
ISR_NOERR 1
ISR_NOERR 2
ISR_NOERR 3
ISR_NOERR 4
ISR_NOERR 5
ISR_NOERR 6
ISR_NOERR 7
ISR_ERR   8
ISR_NOERR 9
ISR_ERR   10
ISR_ERR   11
ISR_ERR   12
ISR_ERR   13
ISR_ERR   14
ISR_NOERR 15
ISR_NOERR 16
ISR_ERR   17
ISR_NOERR 18
ISR_NOERR 19
ISR_NOERR 20
ISR_NOERR 21
ISR_NOERR 22
ISR_NOERR 23
ISR_NOERR 24
ISR_NOERR 25
ISR_NOERR 26
ISR_NOERR 27
ISR_NOERR 28
ISR_NOERR 29
ISR_NOERR 30
ISR_NOERR 31

// IRQ vectors: 32..47
IRQ 32
IRQ 33
IRQ 34
IRQ 35
IRQ 36
IRQ 37
IRQ 38
IRQ 39
IRQ 40
IRQ 41
IRQ 42
IRQ 43
IRQ 44
IRQ 45
IRQ 46
IRQ 47

// Tables of handler addresses
.section .rodata
.align 8
isr_stub_table:
    .quad isr_0, isr_1, isr_2, isr_3, isr_4, isr_5, isr_6, isr_7
    .quad isr_8, isr_9, isr_10, isr_11, isr_12, isr_13, isr_14, isr_15
    .quad isr_16, isr_17, isr_18, isr_19, isr_20, isr_21, isr_22, isr_23
    .quad isr_24, isr_25, isr_26, isr_27, isr_28, isr_29, isr_30, isr_31

irq_stub_table:
    .quad irq_32, irq_33, irq_34, irq_35, irq_36, irq_37, irq_38, irq_39
    .quad irq_40, irq_41, irq_42, irq_43, irq_44, irq_45, irq_46, irq_47

