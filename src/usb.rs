use crate::{SERIAL, USB_DEV};
use cortex_m::interrupt;

pub fn poll_usb() {
    interrupt::free(|cs| {
        if let (Some(usb_dev), Some(serial)) = (
            USB_DEV.borrow(cs).borrow_mut().as_mut(),
            SERIAL.borrow(cs).borrow_mut().as_mut(),
        ) {
            let _ = usb_dev.poll(&mut [serial]);
        }
    });
}
