extern crate alloc;
// use alloc::string::String;
use crate::globals::MAX_MESSAGE_SIZE;
use core::cell::UnsafeCell;
use cortex_m::interrupt::Mutex;
use heapless::Deque;
use heapless::String;
use heapless::Vec;
use rp_pico::hal::sio::Spinlock0;

// 増やしすぎると正常に動作しなくなる たぶん.bssが溢れている
const MAX_BUFFER_SIZE: usize = 8;
const MAX_QUEUE_SIZE: usize = 8;

pub static SHARED_MESSAGE_CORE0_TO_CORE1: Mutex<LockedSharedMessage> =
    Mutex::new(LockedSharedMessage::new());

pub struct LockedSharedMessage {
    data: UnsafeCell<SharedString>,
}

unsafe impl Sync for LockedSharedMessage {}

impl Default for LockedSharedMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedSharedMessage {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(SharedString::new()),
        }
    }

    pub fn write(&self, msg: String<MAX_MESSAGE_SIZE>) {
        let buffer = unsafe { &mut *self.data.get() };
        buffer.push_message(msg);
    }

    pub fn flush(&self) {
        let buffer = unsafe { &mut *self.data.get() };
        buffer.flush_queue();
    }

    pub fn pop(&self) -> Option<String<MAX_MESSAGE_SIZE>> {
        let buffer = unsafe { &mut *self.data.get() };
        buffer.queue_pop()
    }
    pub fn drain_all(&self) -> Vec<String<MAX_MESSAGE_SIZE>, MAX_QUEUE_SIZE> {
        let buffer = unsafe { &mut *self.data.get() };
        buffer.drain_all()
    }
}

// 実バッファ構造体
pub struct SharedString {
    buffer: Deque<String<MAX_MESSAGE_SIZE>, MAX_BUFFER_SIZE>, // 一時バッファ
    queue: Deque<String<MAX_MESSAGE_SIZE>, MAX_QUEUE_SIZE>,   // core1に渡るログキュー
}

impl Default for SharedString {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedString {
    pub const fn new() -> Self {
        Self {
            buffer: Deque::<String<MAX_MESSAGE_SIZE>, MAX_BUFFER_SIZE>::new(),
            queue: Deque::<String<MAX_MESSAGE_SIZE>, MAX_QUEUE_SIZE>::new(),
        }
    }

    pub fn push_message(&mut self, msg: String<MAX_MESSAGE_SIZE>) {
        if let Some(_guard) = Spinlock0::try_claim() {
            self.rotate_buffer();
            self.push_queue(msg);
        } else {
            self.push_buffer(msg);
        }
    }

    fn rotate_buffer(&mut self) {
        while let Some(msg) = self.buffer.pop_front() {
            self.push_queue(msg);
        }
    }

    fn push_buffer(&mut self, msg: String<MAX_MESSAGE_SIZE>) {
        if self.buffer.len() >= MAX_BUFFER_SIZE {
            self.buffer.pop_front();
        }
        let _ = self.buffer.push_back(msg);
    }

    fn push_queue(&mut self, msg: String<MAX_MESSAGE_SIZE>) {
        if self.queue.len() >= MAX_QUEUE_SIZE {
            self.queue.pop_front();
        }
        let _ = self.queue.push_back(msg);
    }

    pub fn flush_queue(&mut self) {
        if let Some(_guard) = Spinlock0::try_claim() {
            self.rotate_buffer();
        }
    }

    pub fn queue_pop(&mut self) -> Option<String<MAX_MESSAGE_SIZE>> {
        self.queue.pop_front()
    }
    pub fn drain_all(&mut self) -> Vec<String<MAX_MESSAGE_SIZE>, MAX_QUEUE_SIZE> {
        let msgs = self
            .queue
            .iter()
            .cloned()
            .collect::<Vec<String<MAX_MESSAGE_SIZE>, MAX_QUEUE_SIZE>>();
        self.queue.clear();
        msgs
    }
}
