use crate::LED_PIN;
use cortex_m::interrupt;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};

#[allow(dead_code)]
pub fn led_on() {
    interrupt::free(|cs| {
        if let Some(pin) = LED_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.set_high().unwrap();
        }
    });
}
#[allow(dead_code)]
pub fn led_off() {
    interrupt::free(|cs| {
        if let Some(pin) = LED_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.set_low().unwrap();
        }
    });
}
#[allow(dead_code)]
pub fn led_toggle() {
    interrupt::free(|cs| {
        if let Some(pin) = LED_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.toggle().unwrap();
        }
    });
}
