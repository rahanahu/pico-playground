use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;
use crate::usb::UsbMessageReciver;
use bsp::hal::{
    gpio::{bank0::Gpio25, FunctionSio, Pin, PullDown, SioOutput},
    multicore::Stack,
    timer::{Alarm0, Alarm1, Alarm2, Alarm3},
};
use usb_device::prelude::*;
use usbd_serial::SerialPort;
// Sharedは同一コア内での割り込みには安全ですが、異なるコア間での共有はできません
// 異なるコア間で共有したい場合はrp2040_halのハードウェアspinlockを使います
pub type Shared<T> = Mutex<RefCell<Option<T>>>;
pub static LED_PIN: Shared<Pin<Gpio25, FunctionSio<SioOutput>, PullDown>> =
    Mutex::new(RefCell::new(None));

pub static USB_DEV: Shared<UsbDevice<'static, bsp::hal::usb::UsbBus>> =
    Mutex::new(RefCell::new(None));

pub static SERIAL: Shared<SerialPort<'static, bsp::hal::usb::UsbBus>> =
    Mutex::new(RefCell::new(None));
pub static USB_RECIEVER: Shared<UsbMessageReciver> = Mutex::new(RefCell::new(None));

pub static ALARM0: Shared<Alarm0> = Mutex::new(RefCell::new(None));
pub static ALARM1: Shared<Alarm1> = Mutex::new(RefCell::new(None));
pub static ALARM2: Shared<Alarm2> = Mutex::new(RefCell::new(None));
pub static ALARM3: Shared<Alarm3> = Mutex::new(RefCell::new(None));
pub static mut CORE1_STACK: Stack<4096> = Stack::new();
pub const MAX_MESSAGE_SIZE: usize = 256; // 最大メッセージサイズ
