// src/core1.rs
use crate::globals::{ALARM2, ALARM3};
use crate::led;
use crate::sharedmessage::SHARED_MESSAGE_CORE0_TO_CORE1;
use cortex_m::asm;
use cortex_m::interrupt;
use defmt::info;
use rp_pico::hal::fugit::MicrosDurationU32;

use rp_pico::hal::{pac, sio::Sio, timer::Alarm};

const TIMER_INTERVAL_100MS: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
const TIMER_INTERVAL_5MS: MicrosDurationU32 = MicrosDurationU32::micros(5_000); // 5ms

pub fn core1_task() {
    info!("Core1 task started");
    // core0で初期化されたクロックとタイマーを使用するために、Peripheralsをstealして取得
    // let mut pac = unsafe { pac::Peripherals::steal() };
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

    // core間通信のテスト
    let raw_sio = unsafe { pac::SIO::steal() };
    let sio = Sio::new(raw_sio);
    let mut fifo = sio.fifo;
    let value = fifo.read_blocking();
    info!("Received value from Core0: {}", value);
    fifo.write_blocking(value + 1); // Core0に値を返す
    info!("Core1 task completed, entering WFI loop");

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
    interrupt::free(|cs| {
        SHARED_MESSAGE_CORE0_TO_CORE1
            .borrow(cs)
            .drain_all()
            .into_iter()
            .for_each(|msg| {
                info!("Core1 received message: {}", msg.as_str());
            });
    })
}
