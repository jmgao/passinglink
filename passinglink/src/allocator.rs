use core::alloc::{GlobalAlloc, Layout};

const SIZE_CLASS_128: usize = 11;
const SIZE_CLASS_256: usize = 10;
const SIZE_CLASS_512: usize = 1;

pub struct Counter {
  current: u8,
  hwm: u8,
}

impl Counter {
  const fn new() -> Counter {
    Counter { current: 0, hwm: 0 }
  }

  pub fn increment(&mut self) {
    self.current += 1;
    if self.current > self.hwm {
      self.hwm = self.current;
    }
  }

  pub fn decrement(&mut self) {
    self.current -= 1;
  }

  pub fn current(&self) -> u8 {
    self.current
  }

  pub fn hwm(&self) -> u8 {
    self.hwm
  }
}

impl core::fmt::Debug for Counter {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "current = {}, hwm = {}", self.current(), self.hwm())
  }
}

static mut BUF_128: [[u8; 128]; SIZE_CLASS_128] = [[0u8; 128]; SIZE_CLASS_128];
static mut USED_128: [bool; SIZE_CLASS_128] = [false; SIZE_CLASS_128];
static mut COUNTER_128: Counter = Counter::new();

static mut BUF_256: [[u8; 256]; SIZE_CLASS_256] = [[0u8; 256]; SIZE_CLASS_256];
static mut USED_256: [bool; SIZE_CLASS_256] = [false; SIZE_CLASS_256];
static mut COUNTER_256: Counter = Counter::new();

static mut BUF_512: [[u8; 512]; SIZE_CLASS_512] = [[0u8; 512]; SIZE_CLASS_512];
static mut USED_512: [bool; SIZE_CLASS_512] = [false; SIZE_CLASS_512];
static mut COUNTER_512: Counter = Counter::new();

pub struct Allocator {}

impl Allocator {
  pub const fn new() -> Allocator {
    Allocator {}
  }
}

pub fn dump_state() {
  unsafe {
    info!("allocator state:");
    info!("  128: {:?}", COUNTER_128);
    info!("  256: {:?}", COUNTER_256);
    info!("  512: {:?}", COUNTER_512);
  }
}

unsafe impl GlobalAlloc for Allocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let size = layout.size().next_power_of_two();
    if size == 128 {
      for (index, used) in USED_128.iter_mut().enumerate() {
        if !*used {
          *used = true;
          COUNTER_128.increment();
          return BUF_128[index].as_mut_ptr();
        }
      }
      panic!("oom: 128");
    } else if size == 256 {
      for (index, used) in USED_256.iter_mut().enumerate() {
        if !*used {
          *used = true;
          COUNTER_256.increment();
          return BUF_256[index].as_mut_ptr();
        }
      }
      panic!("oom: 256");
    } else if size == 512 {
      for (index, used) in USED_512.iter_mut().enumerate() {
        if !*used {
          *used = true;
          COUNTER_512.increment();
          return BUF_512[index].as_mut_ptr();
        }
      }
      panic!("oom: 512");
    } else {
      panic!("unexpected alloc size: {}", layout.size());
    }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    let size = layout.size().next_power_of_two();
    if size == 128 {
      for (index, used) in USED_128.iter_mut().enumerate() {
        if ptr == BUF_128[index].as_mut_ptr() {
          *used = false;
          COUNTER_128.decrement();
          break;
        }
      }
    } else if size == 256 {
      for (index, used) in USED_256.iter_mut().enumerate() {
        if ptr == BUF_256[index].as_mut_ptr() {
          *used = false;
          COUNTER_256.decrement();
          break;
        }
      }
    } else if size == 512 {
      for (index, used) in USED_512.iter_mut().enumerate() {
        if ptr == BUF_512[index].as_mut_ptr() {
          *used = false;
          COUNTER_512.decrement();
          break;
        }
      }
    } else {
      panic!("unexpected dealloc size: {}", layout.size());
    }
  }
}
