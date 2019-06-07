#![no_main]
#![no_std]
#![allow(non_snake_case)]

extern crate panic_semihosting;

use nb::block;
use rtfm::app;
use stm32f1xx_hal::device::USART2;
use stm32f1xx_hal::prelude::*;

use stm32f1xx_hal::serial::{Rx, Serial, Tx};

use stm32f103xx_usb::UsbBus;
use usb_device::bus;
use usb_device::prelude::*;

mod cdc_acm;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut SERIAL_TX: Tx<USART2> = ();
  static mut SERIAL_RX: Rx<USART2> = ();

  static mut USB_DEV: UsbDevice<'static, UsbBus> = ();
  static mut SERIAL: cdc_acm::SerialPort<'static, UsbBus> = ();

  #[init]
  fn init() {
    static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBus>> = None;

    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(8.mhz())
      .sysclk(48.mhz())
      .pclk1(24.mhz())
      .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut afio = device.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

    let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let pin_rx = gpioa.pa3;

    // Create an interface struct for USART1 with 9600 Baud
    let serial = Serial::usart2(
      device.USART2,
      (pin_tx, pin_rx),
      &mut afio.mapr,
      115_200.bps(),
      clocks,
      &mut rcc.apb1,
    );
    let (mut serial_tx, serial_rx) = serial.split();

    for c in b"passinglink v"
      .iter()
      .chain(VERSION.as_bytes())
      .chain(b" initializing...\r\n")
    {
      block!(serial_tx.write(*c)).ok();
    }

    *USB_BUS = Some(UsbBus::usb_with_reset(
      device.USB,
      &mut rcc.apb1,
      &clocks,
      &mut gpioa.crh,
      gpioa.pa12,
    ));

    let serial = cdc_acm::SerialPort::new(USB_BUS.as_ref().unwrap());

    let mut usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x1209, 0xD66B))
      .manufacturer("jmgao")
      .product("Passing Link")
      .serial_number("66C623A66B214BB226X76C236B214A214CC6C236B")
      .device_class(cdc_acm::USB_CLASS_CDC)
      .build();

    usb_dev.force_reset().expect("reset failed");

    USB_DEV = usb_dev;
    SERIAL = serial;
    SERIAL_TX = serial_tx;
    SERIAL_RX = serial_rx;
  }

  #[interrupt(resources = [USB_DEV, SERIAL])]
  fn USB_HP_CAN_TX() {
    usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL);
  }

  #[interrupt(resources = [USB_DEV, SERIAL])]
  fn USB_LP_CAN_RX0() {
    usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL);
  }

  #[idle]
  fn idle() -> ! {
    loop {}
  }
};

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>, serial: &mut cdc_acm::SerialPort<'static, B>) {
  if !usb_dev.poll(&mut [serial]) {
    return;
  }

  let mut buf = [0u8; 8];

  match serial.read(&mut buf) {
    Ok(count) if count > 0 => {
      // Echo back in upper case
      for c in buf[0..count].iter_mut() {
        if 0x61 <= *c && *c <= 0x7a {
          *c &= !0x20;
        }
      }

      serial.write(&buf[0..count]).ok();
    }
    _ => {}
  }
}
