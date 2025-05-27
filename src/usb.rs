extern crate alloc;
use crate::fifomsg::{decode_fifo_msg, encode_cmd, FifoMessageKind};
use crate::globals::MAX_MESSAGE_SIZE;
use crate::globals::{SERIAL, USB_DEV};
use crate::sharedmessage::SHARED_MESSAGE_CORE0_TO_CORE1;
use cortex_m::interrupt;
use defmt::{info, warn};
use heapless::{String, Vec};
use usb_device::bus::UsbBus;
use usbd_serial::SerialPort;

pub fn poll_usb() {
    interrupt::free(|cs| {
        if let (Some(usb_dev), Some(serial)) = (
            USB_DEV.borrow(cs).borrow_mut().as_mut(),
            SERIAL.borrow(cs).borrow_mut().as_mut(),
        ) {
            usb_dev.poll(&mut [serial]);
        }
    });
}

pub struct UsbMessageReciver {
    buffer: Vec<u8, MAX_MESSAGE_SIZE>,
    in_message: bool,
}

impl Default for UsbMessageReciver {
    fn default() -> Self {
        Self::new()
    }
}

impl UsbMessageReciver {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            in_message: false,
        }
    }

    pub fn poll<B: UsbBus>(&mut self, serial: &mut SerialPort<'_, B>) {
        let mut temp = [0u8; 64];
        if let Ok(count) = serial.read(&mut temp) {
            for &b in &temp[..count] {
                match b {
                    b'*' => {
                        self.in_message = true;
                        self.buffer.clear();
                    }
                    b'\n' if self.in_message => {
                        if let Ok(s) = String::<MAX_MESSAGE_SIZE>::from_utf8(self.buffer.clone()) {
                            info!("Message: *{}", s.as_str());
                            self.handle_message(s);
                        } else {
                            warn!("Invalid UTF-8: {:?}", self.buffer[..]);
                        }
                        self.in_message = false;
                        self.buffer.clear();
                    }
                    _ if self.in_message => {
                        if self.buffer.len() < MAX_MESSAGE_SIZE {
                            let _ = self.buffer.push(b);
                        } else {
                            warn!("Message too long, discarding: {:?}", self.buffer[..]);
                            self.in_message = false;
                            self.buffer.clear();
                        }
                    }
                    _ => {} // メッセージ外は無視
                }
            }
        }
    }

    fn handle_message(&self, msg: heapless::String<MAX_MESSAGE_SIZE>) {
        // パースや処理はここに追加
        if let Some(frames) = encode_cmd(msg.clone().as_str()) {
            for frame in &frames {
                let message = decode_fifo_msg(*frame);
                match message {
                    FifoMessageKind::SerialCMD(cmd) => {
                        info!(
                            "Received SerialCMD: cmd:{} ch:{} val:{} ",
                            cmd.cmd(),
                            cmd.channel(),
                            cmd.value()
                        );
                    }
                    FifoMessageKind::PWMCMD(cmd) => {
                        info!("Received PWMCMD: ch:{} val:{} ", cmd.channel(), cmd.value());
                    }
                    FifoMessageKind::VersionCMD(version) => {
                        info!(
                            "Received VersionCMD: version: V{}.{}.{}",
                            version.major(),
                            version.minor(),
                            version.patch()
                        );
                    }
                    _ => (),
                }
            }
        } else {
            warn!("Failed to encode command: {}", msg.as_str());
        }
        interrupt::free(|cs| {
            SHARED_MESSAGE_CORE0_TO_CORE1.borrow(cs).write(msg);
        });
    }
}
