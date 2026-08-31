//
// AArch64 (Cortex-A53) MMU Maintenance Module for AtOS
//
// TODO:
//
// - Test if Data/Instruction Cache Levels are implemented (parse CLIDR_EL1)
//

#![allow(unused)]

use crate::println;

//
// Cache Maintenance Functions
//

// Helper Function for calculating Cache Sets and Ways for AArch64
//
// NOTE:
// See 'dc CISW/CSW/ISW' instructions in Architecture Reference Manual
fn ceil_log2(mut n: u64) -> u32
{
    let mut bits = 0;

    if n <= 1 {
        return 0;
    }

    n -= 1;

    while n != 0 {
        n >>= 1;
        bits += 1;
    }

    bits
}

// Cache Types (Cortex-A53)
pub enum CacheType {
    Instruction,
    DataLevel1,
    DataLevel2,
}

// Instruction Cache operations
pub enum CacheInstructionOperation {
    InvalidateAllInnerSharebale,    // Invalidate all to Point of Unification, Inner Shareable
    InvalidateAll,                  // Invalidate all to Point of Unification
    InvalidateByVA,                 // Invalidate by virtual address to Point of Unification
}

// Data cache operations
//
// NOTE:
// Clean -> Flushes all saved data in Cache to memory system (RAM)
// Invalidate -> Mark all saved data in Cache as invalid
///
// PoC -> Point of Coherency -> Outside of Processor
// PoU -> Point of Unification -> Inside of Processor
// PoP -> Point of Persistence -> NOT supported by Cortex-A53
pub enum CacheDataOperation {
    InvalidateByVAtoPoC,            // Invalidate by VA to PoC
    InvalidateByVAtoPoU,            // Invalidate by VA to PoU
    InvalidateBySetWay,             // Invalidate by set/way (Full Cache)
    CleanByVAtoPoC,                 // Clean by VA to PoC
    CleanByVAtoPoU,                 // Clean by VA to PoU
    CleanBySetWay,                  // Clean by set/way (Full Cache)
    CleanInvalidateByVAtoPoC,       // Clean and invalidate by VA to PoC
    CleanInvalidateBySetWay,        // Clean and invalidate by set/way (Full Cache)
    ZeroByVA,                       // Zero Cache by VA (TODO: NOT implemented)
}

// Cache Information
pub struct CacheInformation {
    cache_type: CacheType,          // Cache Type
    cache_size: usize,              // Total size of Cache
    line_size: u64,                 // Size for a single Cache line
    associativity: u64,             // Number of Ways
    num_sets: u64,                  // Number of Sets
    sets_lo: u8,                    // Sets low bit (for set/way instructions)
    ways_lo: u8,                    // Ways low bit (for set/way instructions)
}

pub struct CacheMaintenance {
    init_done: bool,
    i_cache: CacheInformation,
    d_l1_cache: CacheInformation,
    d_l2_cache: CacheInformation,
}

impl CacheMaintenance {
    // Cache Size Selection Register (CSSELR_EL1)
    const CACHE_DATA: u64 = 0;
    const CACHE_INSTRUCTION: u64 = 1;
    const CACHE_LEVEL1: u64 = 0b000 << 1;
    const CACHE_LEVEL2: u64 = 0b001 << 1;       // Only Data Cache Level 2 (Cortex-A53)

    // Cache Size ID Register (CCSIDR_EL1)
    const CCSIDR_NUMSETS_MASK: u64 = 0x0FFF_E000;
    const CCSIDR_ASSOCIATIVITY_MASK: u64 = 0x0000_1FF8;
    const CCSIDR_LINESIZE_MASK: u64 = 0x0000_0007;

    // Constructor
    pub fn new() -> CacheMaintenance {
        let mut csselr: u64;
        let mut ccsidr: u64;

        /* Instruction Cache */
        let mut i_cache_line_size: u64;
        let mut i_cache_num_sets: u64;
        let mut i_cache_associativity: u64;
        let i_cache_size: usize;

        /* Data Cache Level 1 */
        let mut d_l1_cache_line_size: u64;
        let mut d_l1_cache_num_sets: u64;
        let mut d_l1_cache_associativity: u64;
        let d_l1_cache_size: usize;
        let d_l1_cache_sets_lo: u8;
        let d_l1_cache_ways_lo: u8;

        /* Data Cache Level 2 */
        let mut d_l2_cache_line_size: u64;
        let mut d_l2_cache_num_sets: u64;
        let mut d_l2_cache_associativity: u64;
        let d_l2_cache_size: usize;
        let d_l2_cache_sets_lo: u8;
        let d_l2_cache_ways_lo: u8;


        // Instruction Cache
        csselr = Self::CACHE_INSTRUCTION | Self::CACHE_LEVEL1;

        unsafe {
            core::arch::asm!(
                "msr CSSELR_EL1, {0}",
                "isb",
                "mrs {1}, CCSIDR_EL1",
                "isb",
                in(reg) csselr,
                out(reg) ccsidr);
        };

        i_cache_line_size = 16 << (ccsidr & Self::CCSIDR_LINESIZE_MASK);
        i_cache_num_sets = (ccsidr & Self::CCSIDR_NUMSETS_MASK) >> 13;
        i_cache_num_sets += 1;
        i_cache_associativity = (ccsidr & Self::CCSIDR_ASSOCIATIVITY_MASK) >> 3;
        i_cache_associativity += 1;
        i_cache_size = (i_cache_line_size * i_cache_num_sets * i_cache_associativity) as usize;

        // Data Cache Level 1
        csselr = Self::CACHE_DATA | Self::CACHE_LEVEL1;

        unsafe {
            core::arch::asm!(
                "msr CSSELR_EL1, {0}",
                "isb",
                "mrs {1}, CCSIDR_EL1",
                "isb",
                in(reg) csselr,
                out(reg) ccsidr);
        };

        d_l1_cache_line_size = 16 << (ccsidr & Self::CCSIDR_LINESIZE_MASK);
        d_l1_cache_num_sets = (ccsidr & Self::CCSIDR_NUMSETS_MASK) >> 13;
        d_l1_cache_num_sets += 1;
        d_l1_cache_associativity = (ccsidr & Self::CCSIDR_ASSOCIATIVITY_MASK) >> 3;
        d_l1_cache_associativity += 1;
        d_l1_cache_size = (d_l1_cache_line_size * d_l1_cache_num_sets * d_l1_cache_associativity) as usize;

        // Calculate sets and ways for 'DC CSH/CISH/ISH" instructions for Data Cache Level 1
        d_l1_cache_ways_lo = 32 - ceil_log2(d_l1_cache_associativity) as u8;
        d_l1_cache_sets_lo = ceil_log2(d_l1_cache_line_size) as u8;


        // Data Cache Level 2
        csselr = Self::CACHE_DATA | Self::CACHE_LEVEL2;

        unsafe {
            core::arch::asm!(
                "msr CSSELR_EL1, {0}",
                "isb",
                "mrs {1}, CCSIDR_EL1",
                "isb",
                in(reg) csselr,
                out(reg) ccsidr);
        };

        d_l2_cache_line_size = 16 << (ccsidr & Self::CCSIDR_LINESIZE_MASK);
        d_l2_cache_num_sets = (ccsidr & Self::CCSIDR_NUMSETS_MASK) >> 13;
        d_l2_cache_num_sets += 1;
        d_l2_cache_associativity = (ccsidr & Self::CCSIDR_ASSOCIATIVITY_MASK) >> 3;
        d_l2_cache_associativity += 1;
        d_l2_cache_size = (d_l2_cache_line_size * d_l2_cache_num_sets * d_l2_cache_associativity) as usize;

        // Calculate sets and ways for 'DC CSH/CISH/ISH" instructions for Data Cache Level 2
        d_l2_cache_ways_lo = 32 - ceil_log2(d_l2_cache_associativity) as u8;
        d_l2_cache_sets_lo = ceil_log2(d_l2_cache_line_size) as u8;

        CacheMaintenance {
            init_done: true,

            i_cache: CacheInformation {
                cache_type: CacheType::Instruction,
                cache_size: i_cache_size,
                line_size: i_cache_line_size,
                associativity: i_cache_associativity,
                num_sets: i_cache_num_sets,
                sets_lo: 0,
                ways_lo: 0,
            },

            d_l1_cache: CacheInformation {
                cache_type: CacheType::DataLevel1,
                cache_size: d_l1_cache_size,
                line_size: d_l1_cache_line_size,
                associativity: d_l1_cache_associativity,
                num_sets: d_l1_cache_num_sets,
                sets_lo: d_l1_cache_sets_lo,
                ways_lo: d_l1_cache_ways_lo,
            },

            d_l2_cache: CacheInformation {
                cache_type: CacheType::DataLevel2,
                cache_size: d_l2_cache_size,
                line_size: d_l2_cache_line_size,
                associativity: d_l2_cache_associativity,
                num_sets: d_l2_cache_num_sets,
                sets_lo: d_l2_cache_sets_lo,
                ways_lo: d_l2_cache_ways_lo,
            },
        }
    }

    // Get Total Cache Sizes
    pub fn get_size(&self, cache_type: CacheType) -> usize {
        if self.init_done {
            match cache_type {
                CacheType::Instruction => return self.i_cache.cache_size,
                CacheType::DataLevel1 => return self.d_l1_cache.cache_size,
                CacheType::DataLevel2 => return self.d_l2_cache.cache_size,
            }
        }

        return 0;
    }

    // Print Cache Information
    //
    // TODO:
    // Currently only the Cache Sizes are printed. Add some other functions
    // in the future too.
    pub fn print_cache_info(&self) {
        println!("CPU ICache:         {} KB", self.i_cache.cache_size / 1024);
        println!("CPU DCache Level 1: {} KB", self.d_l1_cache.cache_size / 1024);
        println!("CPU DCache Level 2: {} KB", self.d_l2_cache.cache_size / 1024);
    }

    // Instruction Cache enable
    pub fn icache_enable() {
        unsafe {
            core::arch::asm!(
                "dsb SY",
                "mrs x0, SCTLR_EL1",
                "orr x0, x0, #(1 << 12)",
                "msr SCTLR_EL1, x0",
                "isb",
                out("x0") _,);
        }
    }

    // Instruction Cache disable
    pub fn icache_disable() {
        unsafe {
            core::arch::asm!(
                "dsb SY",
                "mrs x0, SCTLR_EL1",
                "bic x0, x0, #(1 << 12)",
                "msr SCTLR_EL1, x0",
                "isb",
                out("x0") _,);
        }
    }

    // Invalidate Instruction Cache
    //
    // NOTE:
    // 'va' and 'va_range' is only valid for operation CacheInstructionOperation::InvalidateByVA.
    // If 'va_range' is equal to zero then all instruction Cache will be invalidated.
    pub fn icache_invalidate(&self, icache_op: CacheInstructionOperation, va: u64, va_range: usize) {
        unsafe {
            core::arch::asm!(
                "dsb ISH");
        }

        match icache_op {
            CacheInstructionOperation::InvalidateAllInnerSharebale => {
                unsafe {
                    core::arch::asm!(
                        "ic IALLUIS",
                        "dsb ISH");
                };
            }
            CacheInstructionOperation::InvalidateAll => {
                unsafe {
                    core::arch::asm!(
                        "ic IALLU",
                        "dsb SY");
                };
            }
            CacheInstructionOperation::InvalidateByVA => {
                let va_start: u64 = va;
                let va_end: u64;

                // If 'va_range' is equal to zero invalidate full cache lines.
                if va_range == 0 {
                    va_end = va_start + self.i_cache.cache_size as u64;

                    // Execute IC instruction
                    for v in (va_start..va_end).step_by(self.i_cache.line_size as usize) {
                        unsafe {
                            core::arch::asm!(
                                "ic IVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };
                    }
                } else {
                    va_end = va_start + va_range as u64;

                    // Execute IC instruction
                    for v in (va_start..va_end).step_by(self.i_cache.line_size as usize) {
                        unsafe {
                            core::arch::asm!(
                                "ic IVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };
                    }
                }
            }
        }

        unsafe {
            core::arch::asm!(
                "isb");
        }
    }

    // Data Cache enable
    pub fn dcache_enable() {
        unsafe {
            core::arch::asm!(
                "dsb SY",
                "mrs x0, SCTLR_EL1",
                "orr x0, x0, #(1 << 2)",
                "msr SCTLR_EL1, x0",
                "isb",
                out("x0") _,);
        }
    }

    // Data Cache disable
    pub fn dcache_disable() {
        unsafe {
            core::arch::asm!(
                "dsb SY",
                "mrs x0, SCTLR_EL1",
                "bic x0, x0, #(1 << 2)",
                "msr SCTLR_EL1, x0",
                "isb",
                out("x0") _,);
        }
    }

    // Invalidate/Clean Data Caches
    //
    // NOTE:
    // 'va' and 'va_range' is only valid for clean/invalidate operations by VA.
    // If 'va_range' is equal to zero then all data Cache will be invalidated.
    pub fn dcache_invalidate(&self, dcache_op: CacheDataOperation, va: u64, va_range: usize) {
        let va_start: u64 = va;
        let va_end: u64;
        let line_size: usize;

        if self.d_l1_cache.line_size <= self.d_l2_cache.line_size {
            line_size = self.d_l1_cache.line_size as usize;
        } else {
            line_size = self.d_l2_cache.line_size as usize;
        }

        unsafe {
            core::arch::asm!(
                "dsb ISH");
        }

        match dcache_op {
            CacheDataOperation::InvalidateByVAtoPoC => {
                if va_range == 0 {
                    va_end = va_start + line_size as u64;

                    // Execute DC instruction
                    for v in (va_start..va_end).step_by(line_size) {
                        unsafe {
                            core::arch::asm!(
                                "dc IVAC, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };


                    }
                } else {
                    va_end = va_start + va_range as u64;

                    for v in (va_start..va_end).step_by(line_size) {

                        unsafe {
                            core::arch::asm!(
                                "dc IVAC, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };

                    }
                }
            }
            CacheDataOperation::InvalidateByVAtoPoU => {
                if va_range == 0 {
                    va_end = va_start + line_size as u64;

                    // Execute DC instruction
                    for v in (va_start..va_end).step_by(line_size) {
                        unsafe {
                            core::arch::asm!(
                                "dc IVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };


                    }
                } else {
                    va_end = va_start + va_range as u64;

                    for v in (va_start..va_end).step_by(line_size) {

                        unsafe {
                            core::arch::asm!(
                                "dc IVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };

                    }
                }
            }
            CacheDataOperation::CleanByVAtoPoC => {
                if va_range == 0 {
                    va_end = va_start + line_size as u64;

                    // Execute DC instruction
                    for v in (va_start..va_end).step_by(line_size) {
                        unsafe {
                            core::arch::asm!(
                                "dc CVAC, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };


                    }
                } else {
                    va_end = va_start + va_range as u64;

                    for v in (va_start..va_end).step_by(line_size) {

                        unsafe {
                            core::arch::asm!(
                                "dc CVAC, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };

                    }
                }
            }
            CacheDataOperation::CleanByVAtoPoU => {
                if va_range == 0 {
                    va_end = va_start + line_size as u64;

                    // Execute DC instruction
                    for v in (va_start..va_end).step_by(line_size) {
                        unsafe {
                            core::arch::asm!(
                                "dc CVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };


                    }
                } else {
                    va_end = va_start + va_range as u64;

                    for v in (va_start..va_end).step_by(line_size) {

                        unsafe {
                            core::arch::asm!(
                                "dc CVAU, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };

                    }
                }
            }
            CacheDataOperation::CleanInvalidateByVAtoPoC => {
                if va_range == 0 {
                    va_end = va_start + line_size as u64;

                    // Execute DC instruction
                    for v in (va_start..va_end).step_by(line_size) {
                        unsafe {
                            core::arch::asm!(
                                "dc CIVAC, {0}",
                                "dsb ISH",
                                in(reg) v as u64);
                        };


                    }
                } else {
                    va_end = va_start + va_range as u64;

                    for v in (va_start..va_end).step_by(line_size) {

                            unsafe {
                                core::arch::asm!(
                                    "dc CIVAC, {0}",
                                    "dsb ISH",
                                    in(reg) v as u64);
                            };

                    }
                }
            }
            CacheDataOperation::CleanInvalidateBySetWay => {
                let mut ops: u64;

                for w in 0..self.d_l1_cache.associativity {
                    for s in 0..self.d_l1_cache.num_sets {
                        ops = 0;
                        ops |= (w << self.d_l1_cache.ways_lo);
                        ops |= (s << self.d_l1_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc CISW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }

                for w in 0..self.d_l2_cache.associativity {
                    for s in 0..self.d_l2_cache.num_sets {
                        ops = 0b10;
                        ops |= (w << self.d_l2_cache.ways_lo);
                        ops |= (s << self.d_l2_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc CISW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }
            }
            CacheDataOperation::InvalidateBySetWay => {
                let mut ops: u64;

                for w in 0..self.d_l1_cache.associativity {
                    for s in 0..self.d_l1_cache.num_sets {
                        ops = 0;
                        ops |= (w << self.d_l1_cache.ways_lo);
                        ops |= (s << self.d_l1_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc ISW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }

                for w in 0..self.d_l2_cache.associativity {
                    for s in 0..self.d_l2_cache.num_sets {
                        ops = 0b10;
                        ops |= (w << self.d_l2_cache.ways_lo);
                        ops |= (s << self.d_l2_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc ISW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }
            }
            CacheDataOperation::CleanBySetWay => {
                let mut ops: u64;

                for w in 0..self.d_l1_cache.associativity {
                    for s in 0..self.d_l1_cache.num_sets {
                        ops = 0;
                        ops |= (w << self.d_l1_cache.ways_lo);
                        ops |= (s << self.d_l1_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc CSW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }

                for w in 0..self.d_l2_cache.associativity {
                    for s in 0..self.d_l2_cache.num_sets {
                        ops = 0b10;
                        ops |= (w << self.d_l2_cache.ways_lo);
                        ops |= (s << self.d_l2_cache.sets_lo);

                        unsafe {
                            core::arch::asm!(
                                "dc CSW, {0}",
                                "dsb ISH",
                                in(reg) ops as u64);
                        };
                    }
                }
            }
             _ => return,       // TODO
        }

        unsafe {
            core::arch::asm!(
                "isb");
        }
    }
}

//
// MMU Translation Tables Maintenance Functions
//

pub struct TLBMaintenance;

pub enum TLBLevel {
    Level0,
    Level1,
    Level2,
    Level3,
}

pub enum TLBGranuleSize {
    NoInformation,      // No Information provided
    _4KB,
    _16KB,              // NOT implemented on Cortex-A53
    _64KB,
}

pub enum TLBShareability {
    Full,
    Inner,
    Outer,              // NOT implemented on Cortex-A53
}

impl TLBMaintenance {
    const TLBI_ASID_MASK: u64 = 0xFFFF_0000_0000_0000;
    const TLBI_TTL_MASK: u64 = 0x0000_F000_0000_0000;
    const TLBI_VA_MASK: u64 = 0x0000_0FFF_FFFF_FFFF;

    // Invalidate all in MMU cached tables
    pub fn invalidate_all(share: TLBShareability) {
        match share {
            TLBShareability::Full => {
                unsafe {
                    core::arch::asm!(
                        "dsb SY",
                        "tlbi VMALLE1",
                        "dsb SY",
                        "isb");
                };
            }
            TLBShareability::Inner => {
                unsafe {
                    core::arch::asm!(
                        "dsb ISH",
                        "tlbi VMALLE1IS",
                        "dsb ISH",
                        "isb");
                };
            }
            _ => return,        // TODO: Should we output an error message?
        }

    }

    // Invalidate by VA all ASID's
    pub fn invalidate_va_all_asid(granule: TLBGranuleSize, level: TLBLevel, share: TLBShareability, va: u64) {
        let mut v: u64 = 0;
        let mut b_any: bool = false;

        v = (va >> 12) & Self::TLBI_VA_MASK;

        match granule {
            TLBGranuleSize::NoInformation => {
                b_any = true;
            }
            TLBGranuleSize::_4KB => {
                v |= 0b01 << 46;
                b_any = false;
            }
            TLBGranuleSize::_16KB => {
                v |= 0b10 << 46;
                b_any = false;
            }
            TLBGranuleSize::_64KB => {
                v |= 0b11 << 46;
                b_any = false;
            }
        }

        if !b_any {
            match level {
                TLBLevel::Level0 => {
                    v |= 0b00 << 44;
                }
                TLBLevel::Level1 => {
                    v |= 0b01 << 44;
                }
                TLBLevel::Level2 => {
                    v |= 0b10 << 44;
                }
                TLBLevel::Level3 => {
                    v |= 0b11 << 44;
                }
            }
        }

        match share {
            TLBShareability::Full => {
                unsafe {
                    core::arch::asm!(
                        "dsb SY",
                        "tlbi VAAE1, {0}",
                        "dsb SY",
                        "isb",
                        in(reg) v as u64);
                };
            }
            TLBShareability::Inner => {
                unsafe {
                    core::arch::asm!(
                        "dsb ISH",
                        "tlbi VAAE1IS, {0}",
                        "dsb ISH",
                        "isb",
                        in(reg) v as u64);
                };
            }
            _ => return,        // TODO: Should we print an error message here?
        }
    }

    // Invalidate by VA all ASID's last level
    pub fn invalidate_va_all_asid_last(granule: TLBGranuleSize, level: TLBLevel, share: TLBShareability, va: u64) {
        let mut v: u64 = 0;
        let mut b_any: bool;

        v = (va >> 12) & Self::TLBI_VA_MASK;

        match granule {
            TLBGranuleSize::NoInformation => {
                b_any = true;
            }
            TLBGranuleSize::_4KB => {
                v |= 0b01 << 46;
                b_any = false;
            }
            TLBGranuleSize::_16KB => {
                v |= 0b10 << 46;
                b_any = false;
            }
            TLBGranuleSize::_64KB => {
                v |= 0b11 << 46;
                b_any = false;
            }
        }

        if !b_any {
            match level {
                TLBLevel::Level0 => {
                    v |= 0b00 << 44;
                }
                TLBLevel::Level1 => {
                    v |= 0b01 << 44;
                }
                TLBLevel::Level2 => {
                    v |= 0b10 << 44;
                }
                TLBLevel::Level3 => {
                    v |= 0b11 << 44;
                }
            }
        }

        match share {
            TLBShareability::Full => {
                unsafe {
                    core::arch::asm!(
                        "dsb SY",
                        "tlbi VAALE1, {0}",
                        "dsb SY",
                        "isb",
                        in(reg) v as u64);
                };
            }
            TLBShareability::Inner => {
                unsafe {
                    core::arch::asm!(
                        "dsb ISH",
                        "tlbi VAALE1IS, {0}",
                        "dsb ISH",
                        "isb",
                        in(reg) v as u64);
                };
            }
            _ => return,        // TODO: Should we print an error message here?
        }
    }

    // Invalidate by VA and ASID
    pub fn invalidate_va_and_asid(granule: TLBGranuleSize, level: TLBLevel, share: TLBShareability, va: u64, asid: u64) {
        let mut v: u64 = 0;
        let mut b_any: bool;

        v = (va >> 12) & Self::TLBI_VA_MASK;

        match granule {
            TLBGranuleSize::NoInformation => {
                b_any = true;
            }
            TLBGranuleSize::_4KB => {
                v |= 0b01 << 46;
                b_any = false;
            }
            TLBGranuleSize::_16KB => {
                v |= 0b10 << 46;
                b_any = false;
            }
            TLBGranuleSize::_64KB => {
                v |= 0b11 << 46;
                b_any = false;
            }
        }

        if !b_any {
            match level {
                TLBLevel::Level0 => {
                    v |= 0b00 << 44;
                }
                TLBLevel::Level1 => {
                    v |= 0b01 << 44;
                }
                TLBLevel::Level2 => {
                    v |= 0b10 << 44;
                }
                TLBLevel::Level3 => {
                    v |= 0b11 << 44;
                }
            }
        }

        v |= (asid << 48) & Self::TLBI_ASID_MASK;

        match share {
            TLBShareability::Full => {
                unsafe {
                    core::arch::asm!(
                        "dsb SY",
                        "tlbi VAE1, {0}",
                        "dsb SY",
                        "isb",
                        in(reg) v as u64);
                };
            }
            TLBShareability::Inner => {
                unsafe {
                    core::arch::asm!(
                        "dsb ISH",
                        "tlbi VAE1IS, {0}",
                        "dsb ISH",
                        "isb",
                        in(reg) v as u64);
                };
            }
            _ => return,        // TODO: Should we print an error message here?
        }
    }

    // Invalidate by VA and ASID last level
    pub fn invalidate_va_and_asid_last(granule: TLBGranuleSize, level: TLBLevel, share: TLBShareability, va: u64, asid: u64) {
        let mut v: u64 = 0;
        let mut b_any: bool;

        v = (va >> 12) & Self::TLBI_VA_MASK;

        match granule {
            TLBGranuleSize::NoInformation => {
                b_any = true;
            }
            TLBGranuleSize::_4KB => {
                v |= 0b01 << 46;
                b_any = false;
            }
            TLBGranuleSize::_16KB => {
                v |= 0b10 << 46;
                b_any = false;
            }
            TLBGranuleSize::_64KB => {
                v |= 0b11 << 46;
                b_any = false;
            }
        }

        if !b_any {
            match level {
                TLBLevel::Level0 => {
                    v |= 0b00 << 44;
                }
                TLBLevel::Level1 => {
                    v |= 0b01 << 44;
                }
                TLBLevel::Level2 => {
                    v |= 0b10 << 44;
                }
                TLBLevel::Level3 => {
                    v |= 0b11 << 44;
                }
            }
        }

        v |= (asid << 48) & Self::TLBI_ASID_MASK;

        match share {
            TLBShareability::Full => {
                unsafe {
                    core::arch::asm!(
                        "dsb SY",
                        "tlbi VALE1, {0}",
                        "dsb SY",
                        "isb",
                        in(reg) v as u64);
                };
            }
            TLBShareability::Inner => {
                unsafe {
                    core::arch::asm!(
                        "dsb ISH",
                        "tlbi VALE1IS, {0}",
                        "dsb ISH",
                        "isb",
                        in(reg) v as u64);
                };
            }
            _ => return,        // TODO: Should we print an error message here?
        }
    }
}

//
// VA to PA Translation Functions and other
//
pub struct TranslateVAtoPA;

pub enum PASecure {
    Invalid,
    Secure,
    NonSecure,
    Root,
    Realm,
}

pub enum PAShareability {
    Invalid,
    NonShareble,
    OuterShareble,
    InnerShareble,
    Reserved,
}

// Fault Codes
pub enum PAFaultStatus {
    Invalid,
    AddressSizeLevel0,      // Address size fault, level 0 of translation or translation table base register
    AddressSizeLevel1,
    AddressSizeLevel2,
    AddressSizeLevel3,
    TranslationLevel0,
    TranslationLevel1,
    TranslationLevel2,
    TranslationLevel3,
    AccessFlagLevel0,       // Might not be implemented on Cortex-A53
    AccessFlagLevel1,
    AccessFlagLevel2,
    AccessFlagLevel3,
    PermissionLevel0,       // Might not be implemented on Cortex-A53
    PermissionLevel1,
    PermissionLevel2,
    PermissionLevel3,
    TLBConflict,
    Unknown,                // Unknown (not implemented)
}

pub enum VADirection {
    ReadVA,
    WriteVA,
}

pub enum VAStage {
    Stage1,
    Stage1and2,
}

pub struct PAInformation {
    pa_valid: bool,
    // Translation passed (PA is valid)
    share: PAShareability,
    secure: PASecure,
    pa: u64,
    attr: u8,
    // Translation failed (PA is invalid)
    fst: PAFaultStatus,
    ptw: bool,
    s: bool,

}

impl TranslateVAtoPA {
    const PAR_VALID_ATTR_MASK: u64 = 0xFF00_0000_0000_0000;
    const PAR_VALID_PA_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    const PAR_VALID_SH_MASK: u64 = 0x0000_0000_0000_0180;
    const PAR_VALID_NS_MASK: u64 = 0x0000_0000_0000_0200;
    const PAR_VALID_NSE_MASK: u64 = 0x0000_0000_0000_0800;

    const PAR_INVALID_FST_MASK: u64 = 0x0000_0000_0000_007E;
    const PAR_INVALID_PTW_MASK: u64 = 0x0000_0000_0000_0100;
    const PAR_INVALID_S_MASK: u64 = 0x0000_0000_0000_0200;

    // Address Translate for EL0
    pub fn translate_el0(va: u64, stage: VAStage, direction: VADirection) -> PAInformation {
        let par: u64;
        let fst: u64;
        let sh: u64;
        let mut info: PAInformation = PAInformation {
            pa_valid: false,
            pa: 0,
            attr: 0,
            share: PAShareability::Invalid,
            secure: PASecure::Invalid,
            fst: PAFaultStatus::Invalid,
            ptw: false,
            s: false,
        };

        match stage {
            VAStage::Stage1 => {
                match direction {
                    VADirection::ReadVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S1E0R, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                    VADirection::WriteVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S1E0W, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                }
            }
            VAStage::Stage1and2 => {
                match direction {
                    VADirection::ReadVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S12E0R, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                             out(reg) par);
                        };
                    }
                    VADirection::WriteVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S12E0W, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                }
            }
        }

        if (par & 0x1) == 1 {
            fst = (par & Self::PAR_INVALID_FST_MASK) >> 1;

            match fst {
                0b000000 => {
                    info.fst = PAFaultStatus::AddressSizeLevel0;
                }
                0b000001 => {
                    info.fst = PAFaultStatus::AddressSizeLevel1;
                }
                0b000010 => {
                    info.fst = PAFaultStatus::AddressSizeLevel2;
                }
                0b000011 => {
                    info.fst = PAFaultStatus::AddressSizeLevel3;
                }
                0b000100 => {
                    info.fst = PAFaultStatus::TranslationLevel0;
                }
                0b000101 => {
                    info.fst = PAFaultStatus::TranslationLevel1;
                }
                0b000110 => {
                    info.fst = PAFaultStatus::TranslationLevel2;
                }
                0b000111 => {
                    info.fst = PAFaultStatus::TranslationLevel3;
                }
                0b001000 => {
                    info.fst = PAFaultStatus::AccessFlagLevel0;
                }
                0b001001 => {
                    info.fst = PAFaultStatus::AccessFlagLevel1;
                }
                0b001010 => {
                    info.fst = PAFaultStatus::AccessFlagLevel2;
                }
                0b001011 => {
                    info.fst = PAFaultStatus::AccessFlagLevel3;
                }
                0b001100 => {
                    info.fst = PAFaultStatus::PermissionLevel0;
                }
                0b001101 => {
                    info.fst = PAFaultStatus::PermissionLevel1;
                }
                0b001110 => {
                    info.fst = PAFaultStatus::PermissionLevel2;
                }
                0b001111 => {
                    info.fst = PAFaultStatus::PermissionLevel3;
                }
                0b110000 => {
                    info.fst = PAFaultStatus::TLBConflict;
                }
                _ => {
                    info.fst = PAFaultStatus::Unknown;
                }
            }

            if ((par & Self::PAR_INVALID_PTW_MASK) >> 8) == 1 {
                info.ptw = true;
            } else {
                info.ptw = false;
            }

            if ((par & Self::PAR_INVALID_S_MASK) >> 9) == 1 {
                info.s = true;
            } else {
                info.s = false;
            }

        } else {
            info.pa_valid = true;
            info.pa = par & Self::PAR_VALID_PA_MASK;
            info.attr = ((par & Self::PAR_VALID_ATTR_MASK) >> 56) as u8;
            sh = (par & Self::PAR_VALID_SH_MASK) >> 7;

            match sh {
                0b00 => {
                    info.share = PAShareability::NonShareble;
                }
                0b10 => {
                    info.share = PAShareability::OuterShareble;
                }
                0b11 => {
                    info.share = PAShareability::InnerShareble;
                }
                _ => {
                    info.share = PAShareability::Reserved;
                }
            }

            if ((par & Self::PAR_VALID_NSE_MASK) >> 11) == 1 {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::Realm;
                } else {
                    info.secure = PASecure::Root;
                }
            } else {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::NonSecure;
                } else {
                    info.secure = PASecure::Secure;
                }
            }
        }

        info
    }

    // Address Translate for EL1
    pub fn translate_el1(va: u64, stage: VAStage, direction: VADirection) -> PAInformation {
        let par: u64;
        let fst: u64;
        let sh: u64;
        let mut info: PAInformation = PAInformation {
            pa_valid: false,
            pa: 0,
            attr: 0,
            share: PAShareability::Invalid,
            secure: PASecure::Invalid,
            fst: PAFaultStatus::Invalid,
            ptw: false,
            s: false,
        };

        match stage {
            VAStage::Stage1 => {
                match direction {
                    VADirection::ReadVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S1E1R, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                    VADirection::WriteVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S1E1W, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                }
            }
            VAStage::Stage1and2 => {
                match direction {
                    VADirection::ReadVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S12E1R, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                    VADirection::WriteVA => {
                        unsafe {
                            core::arch::asm!(
                                "dmb ISH",
                                "at S12E1W, {0}",
                                "isb",
                                "mrs {1}, PAR_EL1",
                                "isb",
                                in(reg) va,
                                out(reg) par);
                        };
                    }
                }
            }
        }

        if (par & 0x1) == 1 {
            fst = (par & Self::PAR_INVALID_FST_MASK) >> 1;

            match fst {
                0b000000 => {
                    info.fst = PAFaultStatus::AddressSizeLevel0;
                }
                0b000001 => {
                    info.fst = PAFaultStatus::AddressSizeLevel1;
                }
                0b000010 => {
                    info.fst = PAFaultStatus::AddressSizeLevel2;
                }
                0b000011 => {
                    info.fst = PAFaultStatus::AddressSizeLevel3;
                }
                0b000100 => {
                    info.fst = PAFaultStatus::TranslationLevel0;
                }
                0b000101 => {
                    info.fst = PAFaultStatus::TranslationLevel1;
                }
                0b000110 => {
                    info.fst = PAFaultStatus::TranslationLevel2;
                }
                0b000111 => {
                    info.fst = PAFaultStatus::TranslationLevel3;
                }
                0b001000 => {
                    info.fst = PAFaultStatus::AccessFlagLevel0;
                }
                0b001001 => {
                    info.fst = PAFaultStatus::AccessFlagLevel1;
                }
                0b001010 => {
                    info.fst = PAFaultStatus::AccessFlagLevel2;
                }
                0b001011 => {
                    info.fst = PAFaultStatus::AccessFlagLevel3;
                }
                0b001100 => {
                    info.fst = PAFaultStatus::PermissionLevel0;
                }
                0b001101 => {
                    info.fst = PAFaultStatus::PermissionLevel1;
                }
                0b001110 => {
                    info.fst = PAFaultStatus::PermissionLevel2;
                }
                0b001111 => {
                    info.fst = PAFaultStatus::PermissionLevel3;
                }
                0b110000 => {
                    info.fst = PAFaultStatus::TLBConflict;
                }
                _ => {
                    info.fst = PAFaultStatus::Unknown;
                }
            }

            if ((par & Self::PAR_INVALID_PTW_MASK) >> 8) == 1 {
                info.ptw = true;
            } else {
                info.ptw = false;
            }

            if ((par & Self::PAR_INVALID_S_MASK) >> 9) == 1 {
                info.s = true;
            } else {
                info.s = false;
            }

        } else {
            info.pa_valid = true;
            info.pa = par & Self::PAR_VALID_PA_MASK;
            info.attr = ((par & Self::PAR_VALID_ATTR_MASK) >> 56) as u8;
            sh = (par & Self::PAR_VALID_SH_MASK) >> 7;

            match sh {
                0b00 => {
                    info.share = PAShareability::NonShareble;
                }
                0b10 => {
                    info.share = PAShareability::OuterShareble;
                }
                0b11 => {
                    info.share = PAShareability::InnerShareble;
                }
                _ => {
                    info.share = PAShareability::Reserved;
                }
            }

            if ((par & Self::PAR_VALID_NSE_MASK) >> 11) == 1 {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::Realm;
                } else {
                    info.secure = PASecure::Root;
                }
            } else {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::NonSecure;
                } else {
                    info.secure = PASecure::Secure;
                }
            }
        }

        info
    }

    // Address Translate with privilege checks (EL1)
    pub fn translate_el1_pan(va: u64, direction: VADirection) -> PAInformation {
        let par: u64;
        let fst: u64;
        let sh: u64;
        let mut info: PAInformation = PAInformation {
            pa_valid: false,
            pa: 0,
            attr: 0,
            share: PAShareability::Invalid,
            secure: PASecure::Invalid,
            fst: PAFaultStatus::Invalid,
            ptw: false,
            s: false,
        };

        match direction {
            VADirection::ReadVA => {
                unsafe {
                    core::arch::asm!(
                        "dmb ISH",
                        "at S1E1RP, {0}",
                        "isb",
                        "mrs {1}, PAR_EL1",
                        "isb",
                        in(reg) va,
                        out(reg) par);
                };
            }
            VADirection::WriteVA => {
                unsafe {
                    core::arch::asm!(
                        "dmb ISH",
                        "at S1E1WP, {0}",
                        "isb",
                        "mrs {1}, PAR_EL1",
                        "isb",
                        in(reg) va,
                        out(reg) par);
                };
            }
        }

        if (par & 0x1) == 1 {
            fst = (par & Self::PAR_INVALID_FST_MASK) >> 1;

            match fst {
                0b000000 => {
                    info.fst = PAFaultStatus::AddressSizeLevel0;
                }
                0b000001 => {
                    info.fst = PAFaultStatus::AddressSizeLevel1;
                }
                0b000010 => {
                    info.fst = PAFaultStatus::AddressSizeLevel2;
                }
                0b000011 => {
                    info.fst = PAFaultStatus::AddressSizeLevel3;
                }
                0b000100 => {
                    info.fst = PAFaultStatus::TranslationLevel0;
                }
                0b000101 => {
                    info.fst = PAFaultStatus::TranslationLevel1;
                }
                0b000110 => {
                    info.fst = PAFaultStatus::TranslationLevel2;
                }
                0b000111 => {
                    info.fst = PAFaultStatus::TranslationLevel3;
                }
                0b001000 => {
                    info.fst = PAFaultStatus::AccessFlagLevel0;
                }
                0b001001 => {
                    info.fst = PAFaultStatus::AccessFlagLevel1;
                }
                0b001010 => {
                    info.fst = PAFaultStatus::AccessFlagLevel2;
                }
                0b001011 => {
                    info.fst = PAFaultStatus::AccessFlagLevel3;
                }
                0b001100 => {
                    info.fst = PAFaultStatus::PermissionLevel0;
                }
                0b001101 => {
                    info.fst = PAFaultStatus::PermissionLevel1;
                }
                0b001110 => {
                    info.fst = PAFaultStatus::PermissionLevel2;
                }
                0b001111 => {
                    info.fst = PAFaultStatus::PermissionLevel3;
                }
                0b110000 => {
                    info.fst = PAFaultStatus::TLBConflict;
                }
                _ => {
                    info.fst = PAFaultStatus::Unknown;
                }
            }

            if ((par & Self::PAR_INVALID_PTW_MASK) >> 8) == 1 {
                info.ptw = true;
            } else {
                info.ptw = false;
            }

            if ((par & Self::PAR_INVALID_S_MASK) >> 9) == 1 {
                info.s = true;
            } else {
                info.s = false;
            }

        } else {
            info.pa_valid = true;
            info.pa = par & Self::PAR_VALID_PA_MASK;
            info.attr = ((par & Self::PAR_VALID_ATTR_MASK) >> 56) as u8;
            sh = (par & Self::PAR_VALID_SH_MASK) >> 7;

            match sh {
                0b00 => {
                    info.share = PAShareability::NonShareble;
                }
                0b10 => {
                    info.share = PAShareability::OuterShareble;
                }
                0b11 => {
                    info.share = PAShareability::InnerShareble;
                }
                _ => {
                    info.share = PAShareability::Reserved;
                }
            }

            if ((par & Self::PAR_VALID_NSE_MASK) >> 11) == 1 {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::Realm;
                } else {
                    info.secure = PASecure::Root;
                }
            } else {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::NonSecure;
                } else {
                    info.secure = PASecure::Secure;
                }
            }
        }

        info
    }

    // Address Translate without permission checks (EL1)
    pub fn translate_el1_noperm(va: u64) -> PAInformation {
        let par: u64;
        let fst: u64;
        let sh: u64;
        let mut info: PAInformation = PAInformation {
            pa_valid: false,
            pa: 0,
            attr: 0,
            share: PAShareability::Invalid,
            secure: PASecure::Invalid,
            fst: PAFaultStatus::Invalid,
            ptw: false,
            s: false,
        };

        unsafe {
            core::arch::asm!(
                "dmb ISH",
                "at S1E1A, {0}",
                "isb",
                "mrs {1}, PAR_EL1",
                "isb",
                in(reg) va,
                out(reg) par);
        };

        if (par & 0x1) == 1 {
            fst = (par & Self::PAR_INVALID_FST_MASK) >> 1;

            match fst {
                0b000000 => {
                    info.fst = PAFaultStatus::AddressSizeLevel0;
                }
                0b000001 => {
                    info.fst = PAFaultStatus::AddressSizeLevel1;
                }
                0b000010 => {
                    info.fst = PAFaultStatus::AddressSizeLevel2;
                }
                0b000011 => {
                    info.fst = PAFaultStatus::AddressSizeLevel3;
                }
                0b000100 => {
                    info.fst = PAFaultStatus::TranslationLevel0;
                }
                0b000101 => {
                    info.fst = PAFaultStatus::TranslationLevel1;
                }
                0b000110 => {
                    info.fst = PAFaultStatus::TranslationLevel2;
                }
                0b000111 => {
                    info.fst = PAFaultStatus::TranslationLevel3;
                }
                0b001000 => {
                    info.fst = PAFaultStatus::AccessFlagLevel0;
                }
                0b001001 => {
                    info.fst = PAFaultStatus::AccessFlagLevel1;
                }
                0b001010 => {
                    info.fst = PAFaultStatus::AccessFlagLevel2;
                }
                0b001011 => {
                    info.fst = PAFaultStatus::AccessFlagLevel3;
                }
                0b001100 => {
                    info.fst = PAFaultStatus::PermissionLevel0;
                }
                0b001101 => {
                    info.fst = PAFaultStatus::PermissionLevel1;
                }
                0b001110 => {
                    info.fst = PAFaultStatus::PermissionLevel2;
                }
                0b001111 => {
                    info.fst = PAFaultStatus::PermissionLevel3;
                }
                0b110000 => {
                    info.fst = PAFaultStatus::TLBConflict;
                }
                _ => {
                    info.fst = PAFaultStatus::Unknown;
                }
            }

            if ((par & Self::PAR_INVALID_PTW_MASK) >> 8) == 1 {
                info.ptw = true;
            } else {
                info.ptw = false;
            }

            if ((par & Self::PAR_INVALID_S_MASK) >> 9) == 1 {
                info.s = true;
            } else {
                info.s = false;
            }

        } else {
            info.pa_valid = true;
            info.pa = par & Self::PAR_VALID_PA_MASK;
            info.attr = ((par & Self::PAR_VALID_ATTR_MASK) >> 56) as u8;
            sh = (par & Self::PAR_VALID_SH_MASK) >> 7;

            match sh {
                0b00 => {
                    info.share = PAShareability::NonShareble;
                }
                0b10 => {
                    info.share = PAShareability::OuterShareble;
                }
                0b11 => {
                    info.share = PAShareability::InnerShareble;
                }
                _ => {
                    info.share = PAShareability::Reserved;
                }
            }

            if ((par & Self::PAR_VALID_NSE_MASK) >> 11) == 1 {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::Realm;
                } else {
                    info.secure = PASecure::Root;
                }
            } else {
                if ((par & Self::PAR_VALID_NS_MASK) >> 9) == 1 {
                    info.secure = PASecure::NonSecure;
                } else {
                    info.secure = PASecure::Secure;
                }
            }
        }

        info
    }

    // Test a given VA (if mapped)
    pub fn test(va: u64) -> bool {
        let info: PAInformation;

        info = Self::translate_el1_noperm(va);

        if info.pa_valid {
            return true
        }

        return false
    }
}
