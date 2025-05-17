//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use bsp::entry;
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use defmt::*;
use defmt_rtt as _;
use embedded_alloc::LlffHeap as Heap;
use embedded_hal::digital::OutputPin;
use fugit::MicrosDurationU32;
use panic_probe as _;
use rp_pico::hal::timer::Alarm;
// usbシリアル通信サポート
// USB Device support
use usb_device::device::StringDescriptors;
use usb_device::{class_prelude::*, prelude::*};
// USB Communications Class Device support
use usbd_serial::SerialPort;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    pac::interrupt,
    sio::Sio,
    timer::Alarm0,
    watchdog::Watchdog,
    Timer,
};

const USB_VID: u16 = 0x16C0;
const USB_PID: u16 = 0x27DD;
const USB_SERIAL_NUMBER_EN: &str = "picopico";
const USB_MANUFACTURER_EN: &str = "My Company";
const USB_PRODUCT_NAME_EN: &str = "RP2040 USB Serial test";
const USB_POLLING_INTERVAL: MicrosDurationU32 = MicrosDurationU32::micros(2_000); // 2ms  5msにするとusbデバイスが切れる

// グローバルでUSBデバイスとシリアルを共有
static USB_DEV: Mutex<RefCell<Option<UsbDevice<'static, rp_pico::hal::usb::UsbBus>>>> =
    Mutex::new(RefCell::new(None));
static SERIAL: Mutex<RefCell<Option<SerialPort<'static, rp_pico::hal::usb::UsbBus>>>> =
    Mutex::new(RefCell::new(None));
static ALARM0: Mutex<RefCell<Option<Alarm0>>> = Mutex::new(RefCell::new(None));

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    info!("Program start");
    // set heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        #[allow(static_mut_refs)]
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE)
        }
    }
    let mut s = String::from("Hello, ");
    s.push_str("Heap!");
    info!("String: {}", s.as_str());
    info!("Heap: {:#018X}", s.as_ptr() as usize);
    info!("Heap size: {}", s.len());
    info!("Heap capacity: {}", s.capacity());
    info!("Heap address: {:#018X}", s.as_ptr() as usize);

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    // usbポーリングのタイマー割り込みセットアップ
    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let alarm0 = timer.alarm_0().unwrap();
    // Alarm0をグローバルに保存
    cortex_m::interrupt::free(|cs| {
        ALARM0.borrow(cs).replace(Some(alarm0));
    });
    // Alarm0割り込みを有効化し、最初の割り込みをセット（USB_POLLING_INTERVAL後）
    cortex_m::interrupt::free(|cs| {
        if let Some(alarm) = ALARM0.borrow(cs).borrow_mut().as_mut() {
            alarm.schedule(USB_POLLING_INTERVAL).unwrap();
            alarm.enable_interrupt();
        }
    });
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set the USB bus
    let usb_bus = Box::leak(Box::new(UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ))));
    let serial = SerialPort::new(usb_bus);
    let usb_string_desc_en = StringDescriptors::new(LangID::EN_US)
        .manufacturer(USB_MANUFACTURER_EN)
        .product(USB_PRODUCT_NAME_EN)
        .serial_number(USB_SERIAL_NUMBER_EN);
    let usb_string_descs = [usb_string_desc_en];
    // Set a USB device
    let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(USB_VID, USB_PID))
        .strings(&usb_string_descs)
        .expect("Failed to create USB device")
        .device_class(2)
        .build();
    // Set the USB device and serial port to the global variable
    cortex_m::interrupt::free(|cs| {
        USB_DEV.borrow(cs).replace(Some(usb_dev));
        SERIAL.borrow(cs).replace(Some(serial));
    });
    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.
    let mut led_pin = pins.led.into_push_pull_output();

    loop {
        info!("on!");
        led_pin.set_high().unwrap();
        delay.delay_ms(500);
        info!("off!");
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
        cortex_m::interrupt::free(|cs| {
            if let Some(serial) = SERIAL.borrow(cs).borrow_mut().as_mut() {
                let _ = serial.write(b"test\n");
            }
        });
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    // usbポーリングをする大事な割り込みタスク
    cortex_m::interrupt::free(|cs| {
        // Alarm0の割り込みフラグをクリアし、次の割り込みをスケジュール
        if let Some(alarm) = ALARM0.borrow(cs).borrow_mut().as_mut() {
            alarm.clear_interrupt();
            alarm.schedule(USB_POLLING_INTERVAL).ok();
        }
        // USBポーリング
        if let (Some(usb_dev), Some(serial)) = (
            USB_DEV.borrow(cs).borrow_mut().as_mut(),
            SERIAL.borrow(cs).borrow_mut().as_mut(),
        ) {
            let _ = usb_dev.poll(&mut [serial]);
        }
    });
}

// End of file
