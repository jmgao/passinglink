use heapless::consts::U128;
use heapless::spsc::Queue;
use heapless::spsc::SingleCore;

use stm32f1xx_hal::device::USART2;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::serial::{Serial, Tx};

pub struct BufferedSerial {
  tx: Tx<USART2>,
  buffer: Queue<u8, U128, u8, SingleCore>,
}

impl BufferedSerial {
  pub fn poll(&mut self) {
    while let Some(c) = self.buffer.peek() {
      if self.tx.write(c).is_err() {
        self.tx.listen();
        return;
      }
      self.buffer.dequeue();
    }

    self.tx.unlisten();
  }
}

impl BufferedSerial {
  pub fn new<PINS>(serial: Serial<USART2, PINS>) -> Self {
    let (tx, _) = serial.split();
    BufferedSerial {
      tx,
      buffer: unsafe { Queue::u8_sc() },
    }
  }
}

impl core::fmt::Write for BufferedSerial {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    let bytes = s.as_bytes();
    let available = self.buffer.capacity() - self.buffer.len();
    if (available as usize) < bytes.len() {
      return Err(core::fmt::Error);
    }

    for byte in bytes {
      unsafe {
        self.buffer.enqueue_unchecked(*byte);
      }
    }

    self.poll();
    Ok(())
  }
}
