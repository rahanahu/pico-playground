extern crate alloc;

use crate::result::Result;
use alloc::boxed::Box;
use core::alloc::{GlobalAlloc, Layout};
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::cmp::max;
use core::fmt;
use core::ops::DerefMut;
use core::ptr::null_mut;

pub fn round_up_to_nearest_pow2(v: usize) -> Result<usize> {
    1usize
        .checked_shl(usize::BITS - v.wrapping_sub(1).leading_zeros())
        .ok_or("Out of range")
}

pub fn round_up_to_nearest_4bytes(v: usize) -> Result<usize> {
    if v == 0 {
        return Err("Size must be greater than zero");
    }
    v.checked_add(3)
        .and_then(|x| Some(x & !3))
        .ok_or("Out of range")
}
// リンクリストを作ろうの巻
struct Header {
    next_header: Option<Box<Header>>,
    size: usize, //sizeには自分自身も含むので最小はsizeof(Header)
    is_allocated: bool,
    _reserved: usize,
}

const HEADER_SIZE: usize = size_of::<Header>();
#[allow(clippy::assertion_on_constants)]
// ヘッダサイズは16らしい
const _: () = assert!(HEADER_SIZE == 16);
// ヘッダサイズは2の累乗でなければならい
const _: () = assert!(HEADER_SIZE.count_ones() == 1);
pub const LAYOUT_PAGE_4K: Layout = unsafe { Layout::from_size_align_unchecked(4096, 4096) };

impl Header {
    fn can_provide(&self, size: usize, align: usize) -> bool {
        // rough check. auctual size needed may be smaller.
        // HEADER_SIZE*2 => one for allocated region, another for padding.
        self.size >= size + HEADER_SIZE * 2 + align
    }
    fn is_allocated(&self) -> bool {
        self.is_allocated
    }
    fn end_addr(&self) -> usize {
        self as *const Header as usize + self.size
    }
    unsafe fn new_from_addr(addr: usize) -> Box<Header> {
        let header = addr as *mut Header;
        header.write(Header {
            next_header: None,
            size: 0,
            is_allocated: false,
            _reserved: 0,
        });
        Box::from_raw(addr as *mut Header)
    }
    unsafe fn from_allocated_region(addr: *mut u8) -> Box<Header> {
        let header = addr.sub(HEADER_SIZE) as *mut Header;
        Box::from_raw(header)
    }

    // Note: std::alloc::Layout doc says:
    // > All layouts have an associated size and a power-of-two alignment.
    fn provide(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let size = max(round_up_to_nearest_pow2(size).ok()?, HEADER_SIZE);
        // raspi pico は1,2,4バイトアライメント
        // let size = max(round_up_to_nearest_4bytes(size).ok()?, HEADER_SIZE * 2);
        let align = max(align, HEADER_SIZE);
        if self.is_allocated() || !self.can_provide(size, align) {
            // 使われてたり、サイズが不十分な場合何もしない
            None
        } else {
            // Make header for allocated object
            let mut size_used = 0;
            let allocated_addr = (self.end_addr() - size) & !(align - 1);
            let mut header_for_allocated =
                unsafe { Self::new_from_addr(allocated_addr - HEADER_SIZE) };
            header_for_allocated.is_allocated = true;
            // 要求サイズにヘッダは含まれていないので追加
            header_for_allocated.size = size + HEADER_SIZE;
            size_used += header_for_allocated.size;
            header_for_allocated.next_header = self.next_header.take();
            if header_for_allocated.end_addr() != self.end_addr() {
                // Make Header for padding
                let mut header_for_padding =
                    unsafe { Self::new_from_addr(header_for_allocated.end_addr()) };
                header_for_allocated.is_allocated = false;
                header_for_padding.size = self.end_addr() - header_for_allocated.end_addr();
                size_used += header_for_padding.size;
                header_for_padding.next_header = header_for_allocated.next_header.take();
                header_for_allocated.next_header = Some(header_for_padding);
            }
            assert!(self.size >= size_used + HEADER_SIZE);
            self.size -= size_used;
            self.next_header = Some(header_for_allocated);
            Some(allocated_addr as *mut u8)
        }
    }
}

impl Drop for Header {
    fn drop(&mut self) {
        panic!("Header should not be dropped!");
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header @ {:#018X} {{ size: {:#018X}, is_allocated: {} }}",
            self as *const Header as usize,
            self.size,
            self.is_allocated()
        )
    }
}

// K&Rの伝統的FirstFitアルゴリズム
pub struct FirstFitAllocator {
    first_header: RefCell<Option<Box<Header>>>,
}

#[global_allocator]
pub static ALLOCATOR: FirstFitAllocator = FirstFitAllocator {
    first_header: RefCell::new(None),
};

unsafe impl Sync for FirstFitAllocator {}

unsafe impl GlobalAlloc for FirstFitAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_with_options(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let mut region = Header::from_allocated_region(ptr);
        region.is_allocated = false;
        Box::leak(region);
    }
}

impl FirstFitAllocator {
    pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
        let mut header = self.first_header.borrow_mut();
        let mut header = header.deref_mut();
        loop {
            match header {
                Some(e) => match e.provide(layout.size(), layout.align()) {
                    Some(p) => break p,
                    None => {
                        header = e.next_header.borrow_mut();
                        continue;
                    }
                },
                None => {
                    break null_mut::<u8>();
                }
            }
        }
    }
    pub unsafe fn init_heap(&self, heap_addr: usize, heap_size: usize) {
        let mut header = Header::new_from_addr(heap_addr);
        header.size = heap_size;
        *self.first_header.borrow_mut() = Some(header);
    }
}
