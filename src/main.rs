//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
use defmt::*;
use embedded_alloc::LlffHeap as Heap;
use pico_test::core0;
use pico_test::core1;
use rp_pico as bsp;

use bsp::{entry, hal::pac::interrupt};

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    info!("Program start");
    // set Heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        #[allow(static_mut_refs)]
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE)
        }
    }
    core0::main();
}

#[interrupt]
fn TIMER_IRQ_0() {
    core0::handle_timer_irq_0();
}

#[interrupt]
fn TIMER_IRQ_1() {
    core0::handle_timer_irq_1();
}
#[interrupt]
fn TIMER_IRQ_2() {
    // core1のタイマー割り込み
    core1::handle_timer_irq_2()
}
#[interrupt]
fn TIMER_IRQ_3() {
    // core1のタイマー割り込み
    core1::handle_timer_irq_3()
}
