// src/core1.rs
use crate::led;
use core::cell::RefCell;
use cortex_m::asm;
use cortex_m::interrupt::{self, Mutex};
use defmt::info;
use rp_pico::hal::fugit::MicrosDurationU32;
use rp_pico::hal::{
    pac,
    timer::{Alarm, Alarm2, Alarm3},
};

extern "Rust" {
    static mut SHARED_ALARM2: Alarm2;
    static mut SHARED_ALARM3: Alarm3;
}

pub static ALARM2: Mutex<RefCell<Option<Alarm2>>> = Mutex::new(RefCell::new(None));
pub static ALARM3: Mutex<RefCell<Option<Alarm3>>> = Mutex::new(RefCell::new(None));

const TIMER_INTERVAL_100MS: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
const TIMER_INTERVAL_5MS: MicrosDurationU32 = MicrosDurationU32::micros(5_000); // 5ms

pub fn core1_task() {
    info!("Core1 task started");
    // core0で初期化されたクロックとタイマーを使用するために、Peripheralsをstealして取得
    // let mut pac = unsafe { pac::Peripherals::steal() };
    let alarm2 = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SHARED_ALARM2)) };
    let alarm3 = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SHARED_ALARM3)) };
    interrupt::free(|cs| {
        ALARM2.borrow(cs).replace(Some(alarm2));
    });
    interrupt::free(|cs| {
        ALARM3.borrow(cs).replace(Some(alarm3));
    });
    interrupt::free(|cs| {
        if let Some(alarm) = ALARM2.borrow(cs).borrow_mut().as_mut() {
            alarm.schedule(TIMER_INTERVAL_100MS).unwrap();
            alarm.enable_interrupt();
        }
    });
    interrupt::free(|cs| {
        if let Some(alarm) = ALARM3.borrow(cs).borrow_mut().as_mut() {
            alarm.schedule(TIMER_INTERVAL_5MS).unwrap();
            alarm.enable_interrupt();
        }
    });
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_2); // Core1用
    }
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_3); // Core1用
    }

    loop {
        asm::wfi(); // 割り込み待ち
    }
}

pub fn handle_timer_irq_2() {
    interrupt::free(|cs| {
        if let Some(alarm) = ALARM2.borrow(cs).borrow_mut().as_mut() {
            alarm.clear_interrupt();
            alarm.schedule(TIMER_INTERVAL_100MS).ok();
        }
    });
    led::led_toggle();
}

pub fn handle_timer_irq_3() {
    interrupt::free(|cs| {
        if let Some(alarm) = ALARM3.borrow(cs).borrow_mut().as_mut() {
            alarm.clear_interrupt();
            alarm.schedule(TIMER_INTERVAL_5MS).ok();
        }
    });
}
