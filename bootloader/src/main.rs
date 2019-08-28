#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![feature(asm, alloc_error_handler)]

extern crate panic_semihosting;

#[macro_use]
extern crate log;

#[macro_use]
extern crate proper;

#[cfg(not(feature = "no_serial"))]
use core::fmt::Write;

use cortex_m::asm::delay;
use cortex_m::interrupt;

use rtfm::app;
use rtfm::Instant;

use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::gpio::{gpiob, Input, PullUp};

#[cfg(not(feature = "no_serial"))]
use stm32f1xx_hal::serial::{Parity, Serial, StopBits};

use stm32_usbd::{UsbBus, UsbBusType};
use usb_device::bus;
use usb_device::prelude::*;

mod dfu;

#[cfg(not(feature = "no_serial"))]
mod serial;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "no_serial"))]
static mut SERIAL: Option<serial::BufferedSerial> = None;

trait InfallibleInputPin {
  fn is_low(&self) -> bool;
  fn is_high(&self) -> bool;
}

impl<T: embedded_hal::digital::v2::InputPin> InfallibleInputPin for T {
  fn is_low(&self) -> bool {
    if let Ok(result) = embedded_hal::digital::v2::InputPin::is_low(self) {
      result
    } else {
      panic!("failed to read from InputPin");
    }
  }

  fn is_high(&self) -> bool {
    if let Ok(result) = embedded_hal::digital::v2::InputPin::is_high(self) {
      result
    } else {
      panic!("failed to read from InputPin");
    }
  }
}

trait InfallibleOutputPin {
  fn set_low(&mut self);
  fn set_high(&mut self);
}

impl<T: embedded_hal::digital::v2::OutputPin> InfallibleOutputPin for T {
  fn set_low(&mut self) {
    if let Err(_) = embedded_hal::digital::v2::OutputPin::set_low(self) {
      panic!("failed to write to OutputPin");
    }
  }

  fn set_high(&mut self) {
    if let Err(_) = embedded_hal::digital::v2::OutputPin::set_high(self) {
      panic!("failed to write to OutputPin");
    }
  }
}

static mut BOOT_PIN: Option<gpiob::PB12<Input<PullUp>>> = None;

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut USB_DEV: UsbDevice<'static, UsbBusType> = ();
  static mut USB_DFU: dfu::DfuClass = ();

  #[init]
  fn init() {
    static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(8.mhz())
      .sysclk(72.mhz())
      .pclk1(24.mhz())
      .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
    let mut gpioc = device.GPIOC.split(&mut rcc.apb2);
    let mut gpiod = device.GPIOD.split(&mut rcc.apb2);

    unsafe {
      BOOT_PIN = Some(gpiob.pb12.into_pull_up_input(&mut gpiob.crh));
    }

    #[cfg(not(feature = "no_serial"))]
    {
      let mut afio = device.AFIO.constrain(&mut rcc.apb2);
      let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
      let pin_rx = gpioa.pa3;

      let serial_config = stm32f1xx_hal::serial::Config {
        baudrate: 921_600.bps(),
        parity: Parity::ParityNone,
        stopbits: StopBits::STOP1,
      };

      let serial = Serial::usart2(
        device.USART2,
        (pin_tx, pin_rx),
        &mut afio.mapr,
        serial_config,
        clocks,
        &mut rcc.apb1,
      );

      let mut buffered_serial = serial::BufferedSerial::new(serial);

      unsafe {
        let _ = write!(buffered_serial, "\r\n\r\n");
        SERIAL = Some(buffered_serial);
        log::set_logger(SERIAL.as_ref().unwrap()).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
      }
    }

    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    delay(clocks.sysclk().0 / 100);

    let usb_dm = gpioa.pa11;
    let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);
    *USB_BUS = Some(UsbBus::new(device.USB, (usb_dm, usb_dp)));

    let usb_dfu = dfu::DfuClass::new(USB_BUS.as_ref().unwrap());
    let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x1209, 0x214C))
      .manufacturer("jmgao")
      .product("Passing Link Bootloader")
      .serial_number("66C623A66B214BB226X76C236B214A214CC6C236B")
      .device_class(0x00)
      .device_sub_class(0x00)
      .max_power(500)
      .max_packet_size_0(64)
      .build();

    USB_DEV = usb_dev;
    USB_DFU = usb_dfu;
  }

  #[task(priority = 16, schedule = [timer_tick])]
  fn timer_tick() {
    #[cfg(not(feature = "no_serial"))]
    unsafe {
      if let Some(ref mut buffered_serial) = SERIAL {
        buffered_serial.tick();
      }
    }

    schedule.timer_tick(scheduled + 72_000_000.cycles()).unwrap();
  }

  #[interrupt]
  #[cfg(not(feature = "no_serial"))]
  fn USART2() {
    unsafe {
      if let Some(ref mut serial) = SERIAL {
        serial.poll();
      }
    }
  }

  #[interrupt(resources = [USB_DEV, USB_DFU])]
  fn USB_HP_CAN_TX() {
    usb_poll(&mut resources.USB_DEV, &mut resources.USB_DFU);
  }

  #[interrupt(resources = [USB_DEV, USB_DFU])]
  fn USB_LP_CAN_RX0() {
    usb_poll(&mut resources.USB_DEV, &mut resources.USB_DFU);
  }

  extern "C" {
    fn EXTI0();
    fn EXTI1();
  }

  #[idle(schedule = [timer_tick])]
  fn idle() -> ! {
    schedule.timer_tick(Instant::now() + 72_000_000.cycles()).unwrap();

    info!("passinglink bootloader v{} initialized", VERSION);
    loop {
      let should_boot = unsafe {
        BOOT_PIN.as_ref().unwrap().is_low()
      };
      
      if should_boot {
        info!("boot requested, waiting for 100ms and then booting");
        delay(72_000_000 / 10);
        unsafe {
          asm!("
            mov lr, #32769
            bx lr
          ");
        }
      }
    }
  }
};

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>, dfu: &mut dfu::DfuClass) {
  if !usb_dev.poll(&mut [dfu]) {
    return;
  }
}
