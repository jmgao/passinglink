use core::cell::RefCell;
use core::fmt::Write;

use heapless::consts::U512;
use heapless::spsc::Queue;
use heapless::spsc::SingleCore;

use cortex_m::interrupt;
use cortex_m::peripheral::DWT;
use stm32f1xx_hal::device::USART2;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::serial::{Serial, Tx};

struct BufferedSerialState {
  tx: Tx<USART2>,
  buffer: Queue<u8, U512, u16, SingleCore>,

  /// Number of seconds that have elapsed so far.
  seconds: u32,

  /// Cycle count at the last second.
  cycles: u32,
}

impl BufferedSerialState {
  fn poll(&mut self) {
    while let Some(c) = self.buffer.peek() {
      if self.tx.write(c).is_err() {
        self.tx.listen();
        return;
      }
      self.buffer.dequeue();
    }
    self.tx.unlisten();
  }

  fn write(&mut self, s: &str) -> Result<(), ()> {
    let bytes = s.as_bytes();
    let available = self.buffer.capacity() - self.buffer.len();
    if (available as usize) < bytes.len() {
      return Err(());
    }

    for byte in bytes {
      unsafe {
        self.buffer.enqueue_unchecked(*byte);
      }
    }

    self.tx.listen();
    Ok(())
  }

  fn tick(&mut self) {
    self.seconds += 1;
    self.cycles = DWT::get_cycle_count();
  }

  fn elapsed_s(&self) -> u32 {
    self.seconds
  }

  fn elapsed_us(&self) -> u32 {
    let current = DWT::get_cycle_count();
    let cycles = if current >= self.cycles {
      current - self.cycles
    } else {
      core::u32::MAX - self.cycles + current
    };

    cycles / 72
  }
}

impl core::fmt::Write for BufferedSerialState {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.write(s).map_err(|_| core::fmt::Error)
  }
}

pub struct BufferedSerial {
  state: RefCell<BufferedSerialState>,
}

unsafe impl Send for BufferedSerial {}
unsafe impl Sync for BufferedSerial {}

impl BufferedSerial {
  pub fn new<PINS>(serial: Serial<USART2, PINS>) -> Self {
    let (tx, _) = serial.split();
    BufferedSerial {
      state: RefCell::new(BufferedSerialState {
        tx,
        buffer: unsafe { Queue::u16_sc() },
        seconds: 0,
        cycles: DWT::get_cycle_count(),
      }),
    }
  }

  pub fn poll(&self) {
    interrupt::free(|_| {
      let mut state = self.state.borrow_mut();
      state.poll()
    })
  }

  pub fn tick(&mut self) {
    interrupt::free(|_| {
      self.state.borrow_mut().tick();
    })
  }
}

impl core::fmt::Write for BufferedSerial {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    interrupt::free(|_| {
      let mut state = self.state.borrow_mut();
      state.write(s).map_err(|_| core::fmt::Error)
    })
  }
}

impl log::Log for BufferedSerial {
  fn enabled(&self, _: &log::Metadata) -> bool {
    true
  }

  fn log(&self, record: &log::Record) {
    interrupt::free(|_| {
      let mut state = self.state.borrow_mut();
      let s = state.elapsed_s();
      let us = state.elapsed_us();

      if cfg!(feature = "color") {
        const GREEN: &str = "\x1b[32m";
        const RED: &str = "\x1b[31m";
        const ORANGE: &str = "\x1b[31;1m";
        const BRIGHT_WHITE: &str = "\x1b[37;1m";
        const WHITE: &str = "\x1b[37m";
        const GREY: &str = "\x1b[30;1m";
        const RESET: &str = "\x1b[0m";

        let color = match record.level() {
          log::Level::Error => RED,
          log::Level::Warn => ORANGE,
          log::Level::Info => BRIGHT_WHITE,
          log::Level::Debug => WHITE,
          log::Level::Trace => GREY,
        };

        let _ = write!(
          state,
          "{}[{:5}.{:06}] {}{}{}\r\n",
          GREEN,
          s,
          us,
          color,
          record.args(),
          RESET
        );
      } else {
        let _ = write!(state, "[{:5}.{:06}] {}\r\n", s, us, record.args());
      }
    });
  }

  fn flush(&self) {}
}
