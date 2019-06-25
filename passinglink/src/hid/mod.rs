use core::convert::TryFrom;

use usb_device::class_prelude::*;
use usb_device::control::{Recipient, RequestType};
use usb_device::UsbDirection;

mod ps4;
pub use ps4::PS4Hid;

const DESCRIPTOR_TYPE_REPORT: u8 = 0x22;

#[derive(Prim, Clone, Copy, Debug, PartialEq)]
#[prim(ty = "u8")]
pub enum HidRequest {
  GetReport = 0x01,
  GetIdle = 0x02,
  GetProtocol = 0x03,

  SetReport = 0x09,
  SetIdle = 0x0a,
  SetProtocol = 0x0b,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HidReportType {
  Input,
  Output,
  Feature,
  Reserved(u8),
}

impl From<u8> for HidReportType {
  fn from(x: u8) -> HidReportType {
    match x {
      1 => HidReportType::Input,
      2 => HidReportType::Output,
      3 => HidReportType::Feature,
      _ => HidReportType::Reserved(x),
    }
  }
}

pub trait Hid {
  fn report_descriptor(&self) -> &[u8];
  fn get_report(&mut self, report_type: HidReportType, report_id: u8, length: Option<u16>) -> Result<&[u8], ()>;
  fn set_report(&mut self, report_type: HidReportType, report_id: u8, data: &[u8]) -> Result<(), ()>;
}

pub struct HidClass<'a, H: Hid, B: UsbBus> {
  hid: H,
  interface: InterfaceNumber,
  ep_in: EndpointIn<'a, B>,
  ep_out: EndpointOut<'a, B>,
  idle: u8,
}

impl<H: Hid, B: UsbBus> HidClass<'_, H, B> {
  pub fn new(hid: H, alloc: &UsbBusAllocator<B>) -> HidClass<'_, H, B> {
    let ep_in = alloc
      .alloc(
        Some(EndpointAddress::from_parts(4, UsbDirection::In)),
        EndpointType::Interrupt,
        64,
        1,
      )
      .unwrap();

    // TODO: Should we be allocating ep_out in HidDevice instead?
    // TODO: Actually read from ep_out and forward input to HidDevice.
    let ep_out = alloc
      .alloc(
        Some(EndpointAddress::from_parts(3, UsbDirection::Out)),
        EndpointType::Interrupt,
        64,
        1,
      )
      .unwrap();

    HidClass {
      hid,
      interface: alloc.interface(),
      ep_in,
      ep_out,
      idle: 0,
    }
  }

  pub fn send(&mut self) {
    let data = self
      .hid
      .get_report(HidReportType::Input, 0, None)
      .expect("failed to get report");
    let result = self.ep_in.write(data);
    if let Ok(len) = result {
      if len != data.len() {
        error!(
          "write returned short: expected to write {} bytes, actually wrote {}",
          data.len(),
          len
        );
      }
    }
  }

  fn get_report(&mut self, xfer: ControlIn<B>) {
    debug!("HidClass::get_report");
    let req = xfer.request();
    let [report_type, report_id] = req.value.to_be_bytes();
    let report_type = HidReportType::from(report_type);
    match self.hid.get_report(report_type, report_id, Some(req.length)) {
      Ok(data) => xfer.accept_with(data).unwrap(),
      Err(()) => xfer.reject().unwrap(),
    };
  }

  fn set_report(&mut self, xfer: ControlOut<B>) {
    debug!("HidClass::set_report");
    let req = xfer.request();
    let [report_type, report_id] = req.value.to_be_bytes();
    let report_type = HidReportType::from(report_type);
    match self.hid.set_report(report_type, report_id, xfer.data()) {
      Ok(()) => xfer.accept().unwrap(),
      Err(()) => xfer.reject().unwrap(),
    };
  }

  fn get_idle(&mut self, xfer: ControlIn<B>) {
    let req = xfer.request();
    let [should_be_zero, report_id] = req.value.to_be_bytes();
    if should_be_zero != 0 {
      error!("HidClass::get_idle({}, {}): invalid request", should_be_zero, report_id);
      xfer.reject().unwrap();
    } else {
      if report_id != 0 {
        warn!("HidClass::get_idle unimplemented for nonzero report id");
      }
      xfer.accept_with(core::slice::from_ref(&self.idle)).unwrap();
    }
  }

  fn set_idle(&mut self, xfer: ControlOut<B>) {
    let req = xfer.request();
    let [idle, report_id] = req.value.to_be_bytes();
    warn!("HidClass::set_idle({}, {}) mostly unimplemented", idle, report_id);
    if report_id == 0 {
      self.idle = idle;
    }
    xfer.accept().unwrap();
  }

  fn get_protocol(&mut self, xfer: ControlIn<B>) {
    error!("HidClass::get_protocol");
    xfer.reject().unwrap();
  }

  fn set_protocol(&mut self, xfer: ControlOut<B>) {
    error!("HidClass::get_protocol({:?})", xfer.data());
    xfer.reject().unwrap();
  }
}

impl<H: Hid, B: UsbBus> UsbClass<B> for HidClass<'_, H, B> {
  fn poll(&mut self) {}

  fn reset(&mut self) {
    info!("HidDevice::reset");
    self.idle = 0;
  }

  #[rustfmt::skip]
  fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> usb_device::Result<()> {
    debug!("HidClass::get_configuration_descriptors");
    writer.interface(
      self.interface,
      0x03, // HID class
      0x00, // No subclass
      0x00, // No protocol
    )?;

    let report_descriptor = self.hid.report_descriptor();
    let descriptor_len = (report_descriptor.len() as u16).to_le_bytes();
    writer.write(
      0x21,                                   // bDescriptorType (HID)
      &[
        0x11, 0x1,                            // bcdHID (1.11)
        0,                                    // bCountryCode
        1,                                    // bNumDescriptors
        DESCRIPTOR_TYPE_REPORT,               // bDescriptorType (Report)
        descriptor_len[0], descriptor_len[1], // bDescriptorLength
      ],
    )?;

    writer.endpoint(&self.ep_in)?;
    writer.endpoint(&self.ep_out)?;

    Ok(())
  }

  fn get_string(&self, _index: StringIndex, _lang_id: u16) -> Option<&str> {
    None
  }

  fn control_in(&mut self, xfer: ControlIn<B>) {
    let req = *xfer.request();
    if req.recipient != Recipient::Interface {
      return;
    }

    match req.request_type {
      RequestType::Standard => match req.request {
        control::Request::GET_DESCRIPTOR => {
          let (descriptor_type, index) = req.descriptor_type_index();
          if descriptor_type == DESCRIPTOR_TYPE_REPORT && index == 0 {
            debug!("fulfilling GET_DESCRIPTOR(Report, {})", index);
            let descriptor = self.hid.report_descriptor();
            xfer.accept_with(descriptor).ok();
          } else {
            warn!("unhandled GET_DESCRIPTOR({:#x}, {})", descriptor_type, index);
          }
        }

        _ => {
          warn!(
            "unhandled standard in request: request = {}, value = {}, index = {}, length = {}",
            req.request, req.value, req.index, req.length
          );
        }
      },

      RequestType::Class => {
        if let Ok(request) = HidRequest::try_from(req.request) {
          match request {
            HidRequest::GetReport => self.get_report(xfer),
            HidRequest::GetIdle => self.get_idle(xfer),
            HidRequest::GetProtocol => self.get_protocol(xfer),
            _ => warn!("unhandled class in request type: {:?}", request),
          }
        } else {
          warn!("unexpected class in request: {}", req.request);
        }
      }

      RequestType::Vendor => warn!("unhandled vendor in request: {:?}", req),
      RequestType::Reserved => warn!("unhandled reserved in request: {:?}", req),
    }
  }

  fn control_out(&mut self, xfer: ControlOut<B>) {
    let req = *xfer.request();
    if req.recipient != Recipient::Interface {
      return;
    }

    match req.request_type {
      RequestType::Standard => warn!("unhandled standard out request: {:?}", req),
      RequestType::Class => {
        if let Ok(request) = HidRequest::try_from(req.request) {
          match request {
            HidRequest::SetReport => self.set_report(xfer),
            HidRequest::SetIdle => self.set_idle(xfer),
            HidRequest::SetProtocol => self.set_protocol(xfer),
            _ => warn!("unhandled class out request type: {:?}", request),
          }
        } else {
          warn!("unexpected class out request: {}", req.request);
        }
      }
      RequestType::Vendor => warn!("unhandled vendor out request: {:?}", req),
      RequestType::Reserved => warn!("unhandled reserved out request: {:?}", req),
    }
  }
}
