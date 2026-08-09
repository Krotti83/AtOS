.section ".text.boot"
.global _start
.func _start
.type _start,%function
_start:
    mrs x0, S3_1_C15_C2_0       /* Read CPUACTLR_EL1 */
    orr x0, x0, #(1 << 24)      /* Fix Cortex-A53 errata 836870 */
    orr x0, x0, #(1 << 44)      /* Fix Cortex-A53 errata 855873 */
    msr S3_1_C15_C2_0, x0

    // MMU setup
    ldr     x7, =0x3FFFFFFF // i have setup mmu for EL1 kernel space such that physical address = virtual address & 0x3FFFFFFF.
                           // this is because the paging configuration essentially offsets entire memory space to 
                          // virtual address 0xFFFF_FF80_0000_0000.

    // setting configurations 
    ldr     x0, =TCR_EL1_VALUE
    and     x0, x0, x7          // link time addresses are virtual. i have things designed so & 3FFFFFFF gives physical address (for ttbr1)
    ldr     x0, [x0]
    msr     TCR_EL1, x0
    ldr     x0, =MAIR_EL1_VALUE
    and     x0, x0, x7          // to physical address
    ldr     x0, [x0]
    msr     MAIR_EL1, x0
    isb

    // linking up the kernel page tables from paging.rs
    ldr     x1, =PAGE_TABLE_KERNEL_L2_0
    and     x1, x1, x7          // to physical address
    orr     x1, x1, #0b11
    ldr     x2, =PAGE_TABLE_KERNEL_L2_1  
    and     x2, x2, x7          // to physical address
    orr     x2, x2, #0b11
    ldr     x0, =PAGE_TABLE_KERNEL_L1
    and     x0, x0, x7          // to physical address
    str     x1, [x0]
    str     x2, [x0, #8]
    dsb     ishst

    // now loading the page tables
    msr     TTBR1_EL1, x0       // load page table!
    msr     TTBR0_EL1, x0       // load the identical page table for ttbr0!
    isb

    // we just loaded the same page table in ttbr0. because even though our final objective is for kernel to 
    // work in ttbr1 virtual address space, right now once we turn mmu on, the PC is STILL at the old physical address. 
    // so we need to load the same page table in ttbr0 so that the current physical address is still valid after mmu is turned on. 
    // then we can do a really huge jump to the correct place in ttbr1 virtual address space after mmu is turned on.

    // turning on the MMU
    mrs     x0, SCTLR_EL1
    ldr     x1, =SCTLR_EL1_ENABLE_MMU
    and     x1, x1, x7          // to physical address
    ldr     x1, [x1]
    orr     x0, x0, x1
    msr     SCTLR_EL1, x0
    isb

    // now, accoeding to our mmu design, kernel is supposed to be at 0xffffff8000080000.
    // ttbr0 is for user space we just set it right now to facilitate what we're about to do.
    // we'll now jump to the correct place!

    adr x16, transition_to_el1
    br  x16 

transition_to_el1:

    // checking current exception level
    mrs     x0, CurrentEL
    cmp     x0, #0x8 // EL2?
    b.ne    el1_start
    
    // EL stack pointer
    ldr     x0, =_stack_top
    msr     SP_EL1, x0
    
    mov     x0, #(1 << 31)      // bit 31 selects Aarch32/64
    msr     HCR_EL2, x0         // HCR_EL2 is like "a rulebook that EL2 writes for EL1"

    /* Enable AArch64 timer access for EL1 */
    mrs x0, CNTHCTL_EL2
    orr x0, x0, #(1 << 0)
    orr x0, x0, #(1 << 1)
    msr CNTHCTL_EL2, x0
    msr CNTVOFF_EL2, xzr

    // set SPSR_EL2 to EL1h, basically once we ERET, we must switch to EL1h mode
    mov     x0, #(0b00101)      // EL1h 
    orr     x0, x0, #(0b1111 << 6) // mask all exceptions (D, A, I, F)
    msr     SPSR_EL2, x0

    // finally set the ERET PC
    adr     x0, el1_start
    msr     ELR_EL2, x0

    eret // insane

el1_start:

    // set stack pointer
    ldr     x0, =_stack_top
    mov     sp, x0

    bl rt_setup

    // load exception vector table for EL1
    ldr     x0, =el1_vectors
    msr     VBAR_EL1, x0
    isb

    // diables fp/simd interrupts 
    mrs     x0, CPACR_EL1
    orr     x0, x0, #(3 << 20)
    msr     CPACR_EL1, x0
    isb

    /* Clean and invalidate Caches */
    bl icache_clean_invalidate
    bl dcache_l1_clean_invalidate
    bl dcache_l2_clean_invalidate

    mrs x0, SCTLR_EL1       /* Read SCTLR_EL1 register */
    orr x0, x0, #(1 << 2)   /* Enable D-Cache */
    orr x0, x0, #(1 << 12)  /* Enable I-Cache */
    msr SCTLR_EL1, x0       /* Write SCTLR_EL1 register */
    isb

    ldr     x16, =_rust_main // without this roundabout way, it would do a relative jump, allegedly.
    br      x16                  // Absolute jump directly into higher-half TTBR1 space!
.pool
.endfunc

/**
 * Runtime setup (zero .bss)
 */
.func rt_setup
.type rt_setup,%function
rt_setup:
    ldr x0, =_bss_start
    ldr x1, =_bss_end
    sub x2, x1, x0
    cmp x2, #0
    beq 2f
1:
    strb wzr, [x0], #1
    sub x2, x2, #1
    cmp x2, #0
    bne 1b
2:
    ret
.pool
.endfunc

/**
 * Clean and invalidate instruction cache
 */
.func icache_clean_invalidate
.global icache_clean_invalidate
.type icache_clean_invalidate,%function
icache_clean_invalidate:
    ic IALLUIS
    dsb SY
    isb
    ret
.pool
.endfunc

/**
 * Clean and invalidate level 1 data cache
 */
.func dcache_l1_clean_invalidate
.global dcache_l1_clean_invalidate
.type dcache_l1_clean_invalidate,%function
dcache_l1_clean_invalidate:
    mov x0, xzr
    msr CSSELR_EL1, x0
    mrs x4, CCSIDR_EL1
    and x1, x4, #0x7
    add x1, x1, #0x4
    ldr x3, =0x7FFF
    and x2, x3, x4, lsr #13
    ldr x3, =0x3FF
    and x3, x3, x4, lsr #3
    clz w4, w3
    mov x5, #0
1:
    mov x6, #0
2:
    lsl x7, x5, x4
    orr x7, x0, x7
    lsl x8, x6, x1
    orr x7, x7, x8
    dc CISW, x7
    add x6, x6, #1
    cmp x6, x2
    ble 2b
    add x5, x5, #1
    cmp x5, x3
    ble 1b
    dsb SY
    isb
    ret
.pool
.endfunc

/**
 * Clean and invalidate level 2 data cache
 */
.func dcache_l2_clean_invalidate
.global dcache_l2_clean_invalidate
.type dcache_l2_clean_invalidate,%function
dcache_l2_clean_invalidate:
    mov x0, xzr
    orr x0, x0, #(1 << 1)
    msr CSSELR_EL1, x0
    mrs x4, CCSIDR_EL1
    and x1, x4, #0x7
    add x1, x1, #0x4
    ldr x3, =0x7FFF
    and x2, x3, x4, lsr #13
    ldr x3, =0x3FF
    and x3, x3, x4, lsr #3
    clz w4, w3
    mov x5, #0
1:
    mov x6, #0
2:
    lsl x7, x5, x4
    orr x7, x0, x7
    lsl x8, x6, x1
    orr x7, x7, x8
    dc CISW, x7
    add x6, x6, #1
    cmp x6, x2
    ble 2b
    add x5, x5, #1
    cmp x5, x3
    ble 1b
    dsb SY
    isb
    ret
.pool
.endfunc
