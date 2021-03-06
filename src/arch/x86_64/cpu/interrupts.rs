//
//  SOS: the Stupid Operating System
//  by Hawk Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2015 Hawk Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
//! 64-bit Interrupt Descriptor Table implementation.
//!
//! Refer to section 6.10 of the _Intel® 64 and IA-32 Architectures
//! Software Developer’s Manual_ for more information.
use core::mem;
use spin::Mutex;
use super::{Registers, DTable, segment};

#[path = "../../x86_all/interrupts.rs"] mod interrupts_all;
#[path = "../../x86_all/pics.rs"] pub mod pics;
pub use self::interrupts_all::*;

//==------------------------------------------------------------------------==
// Interface into ASM interrupt handling
extern {
    /// Offset of the 64-bit GDT main code segment.
    /// Exported by `boot.asm`
    static gdt64_offset: u16;

    /// Array of interrupt handlers from ASM
    static int_handlers: [Option<Handler>; IDT_ENTRIES];
}

/// State stored when handling an interrupt.
#[allow(dead_code)]
#[repr(C, packed)]
struct InterruptCtx64 {  /// callee-saved registers
                         registers: Registers
                       , /// interrupt ID number
                         int_id:  u32
                       , __pad_1: u32
                       , /// error number
                         err_no:  u32
                       , __pad_2: u32
                       }

impl InterruptContext for InterruptCtx64 {
    type Registers = Registers;
    // All these inline functions are basically just faking
    // object orientation in a way the Rust compiler understands
    #[inline] fn registers(&self) -> Self::Registers { self.registers }
    #[inline] fn err_no(&self) -> u32 { self.err_no }
    #[inline] fn int_id(&self) -> u32 { self.int_id }
}


//==------------------------------------------------------------------------==
// 64-bit implementation of the IDT gate trait

/// An IDT entry is called a gate.
///
/// Based on code from the OS Dev Wiki
/// http://wiki.osdev.org/Interrupt_Descriptor_Table#Structure
///
/// Refer also to "6.14.1 64-Bit Mode IDT"  and "Table 3-2. System-Segment and
/// Gate-Descriptor Types" in the _Intel® 64 and IA-32 Architectures
/// Software Developer’s Manual_
#[repr(C, packed)]
#[derive(Copy,Clone)]
struct Gate64 { /// bits 0 - 15 of the offset
                offset_lower: u16
              , /// code segment selector (GDT or LDT)
                selector: segment::Selector
              , /// always zero
                zero: u8
              , /// indicates the gate's type and attributes.
                /// the second half indicates the type:
                ///   + `0b1100`: Call gate
                ///   + `0b1110`: Interrupt gate
                ///   + `0b1111`: Trap Gate
                type_attr: u8
              , /// bits 16 - 31 of the offset
                offset_mid: u16
              , /// bits 32 - 63 of the offset
                offset_upper: u32
              , /// always zero (according to the spec, this is "reserved")
                reserved: u32
              }

impl Gate64 {
    /// Creates a new IDT gate marked as `absent`.
    ///
    /// This is basically just for filling the new IDT table
    /// with valid (but useless) gates upon init.
    ///
    /// This would be in the `Gate` trait, but this has to be a `const fn` so
    /// that it can be used in static initializers, and trait functions cannot
    /// be `const`.
    const fn absent() -> Self {
        Gate64 { offset_lower: 0
               , selector: segment::Selector::from_raw(0)
               , zero: 0
               , type_attr: GateType::Absent as u8
               , offset_mid: 0
               , offset_upper: 0
               , reserved: 0
               }
    }
}

impl Gate for Gate64 {

    /// Creates a new IDT gate pointing at the given handler function.
    ///
    /// The `handler` function must have been created with valid interrupt
    /// calling conventions.
    fn from_handler(handler: Handler) -> Self {
        unsafe { // trust me on this.
                 // `mem::transmute()` is glorious black magic
            let (low, mid, high): (u16, u16, u32)
                = mem::transmute(handler);

            Gate64 { offset_lower: low
                   , selector: segment::Selector::new(gdt64_offset)
                   , zero: 0
                   // Bit 7 is the present bit
                   // Bits 4-0 indicate this is an interrupt gate
                   , type_attr: GateType::Interrupt as u8
                   , offset_mid: mid
                   , offset_upper: high
                   , reserved: 0
                   }
        }
    }
}

//==------------------------------------------------------------------------==
// 64-bit implementation of the IDT trait
struct Idt64([Gate64; IDT_ENTRIES]);

impl Idt for Idt64 {
    // type Ptr = IdtPtr<Self>;
    type Ctx = InterruptCtx64;
    type GateSize = Gate64;

    /// Get the IDT pointer struct to pass to `lidt`
    // fn get_ptr(&self) -> DTablePtr<Self> {
    //     IdtPtr { limit: (mem::size_of::<Gate64>() * IDT_ENTRIES) as u16
    //            , base:  self as *const Idt64
    //            }
    // }

    /// Add an entry for the given handler at the given index
    fn add_gate(&mut self, index: usize, handler: Handler) {
        self.0[index] = Gate64::from_handler(handler)
    }

    /// Assembly interrupt handlers call into this
    extern "C" fn handle_interrupt(state: &Self::Ctx) {
        let id = state.int_id();
        match id {
            // interrupts 0 - 16 are CPU exceptions
            0x00...0x0f => Self::handle_cpu_exception(state)
            // System timer
          , 0x20 => { /* TODO: make this work */ }
            // Keyboard
          , 0x21 => { /* TODO: make this work */ }
          , _ => panic!("Unknown interrupt: #{} Sorry!", id)
        }
        // send the PICs the end interrupt signal
        unsafe { pics::end_pic_interrupt(id as u8); }
    }
}

impl DTable for Idt64 {
    #[inline] unsafe fn load(&self) {
        asm!(  "lidt [$0]"
            :: "A"(self.get_ptr())
            :: "intel" );
    }
}

// impl IdtPtrOps for IdtPtr<Idt64> {
//     /// Load the IDT at the given location.
//     /// This just calls `lidt`.
//     unsafe fn load(&self) {
//         asm!(  "lidt ($0)"
//             :: "{rax}"(self)
//             :: "volatile" );
//     }
// }

/// Global Interrupt Descriptor Table instance
/// Our global IDT.
static IDT: Mutex<Idt64>
    = Mutex::new(Idt64([Gate64::absent(); IDT_ENTRIES]));

pub fn initialize() {
    let mut idt = IDT.lock();

    // TODO: load interrupts into IDT

    unsafe {
        idt.load();                 // Load the IDT pointer
        pics::initialize();         // initialize the PICs
        Idt64::enable_interrupts(); // enable interrupts
    }
}
