use core::convert::TryFrom;

use usb_device::class_prelude::*;
use usb_device::control::{Recipient, RequestType};
use usb_device::UsbDirection;

const fn dfu_can_download() -> bool {
  true
}

const fn dfu_can_upload() -> bool {
  true
}

const fn dfu_block_begin() -> usize {
  // Carve out the first 32kB for the bootloader.
  0x8000_0000 + 32 * 1024
}

trait State
where
  Self: core::marker::Sized,
{
  fn download<B: UsbBus>(self, xfer: ControlIn<B>) -> DfuState {
    DfuState::Failed
  }
}

struct Idle {}

// A.2.3 dfuIDLE
impl State for Idle {
  fn download<B: UsbBus>(self, xfer: ControlIn<B>) -> DfuState {
    if dfu_can_download() {
      DfuState::Failed
    } else {
      xfer.reject();
      DfuState::Failed
    }
  }
}

pub enum DfuState {
  Idle(Idle),
  DownloadSync,
  DownloadBusy,
  DownloadIdle,
  ManifestSync,
  Manifest,
  ManifestWaitReset,
  UploadIdle,

  // aka "Error", but that collides with the Error type.
  Failed,
}

impl DfuState {
  fn idle() -> DfuState {
    DfuState::Idle(Idle {})
  }

  fn byte_value(&self) -> u8 {
    match &self {
      DfuState::Idle(_) => 2,
      DfuState::DownloadSync => 3,
      DfuState::DownloadBusy => 4,
      DfuState::DownloadIdle => 5,
      DfuState::ManifestSync => 6,
      DfuState::Manifest => 7,
      DfuState::ManifestWaitReset => 8,
      DfuState::UploadIdle => 9,
      DfuState::Failed => 10,
    }
  }
}

#[derive(Prim, Clone, Copy, Debug, PartialEq)]
#[prim(ty = "u8")]
pub enum DfuRequest {
  // Shouldn't happen: we're already in DFU mode.
  Detach = 0x00,

  Download = 0x01,
  Upload = 0x02,
  GetStatus = 0x03,
  ClearStatus = 0x04,
  GetState = 0x05,
  Abort = 0x06,
}

pub struct DfuClass {
  // Host to device.
  support_download: bool,

  // Device to host.
  support_upload: bool,

  interface: InterfaceNumber,

  status: u8,
  state: DfuState,
}

impl DfuClass {
  pub fn new<B: UsbBus>(alloc: &UsbBusAllocator<B>) -> DfuClass {
    DfuClass {
      support_download: true,
      support_upload: true,
      interface: alloc.interface(),
      status: 0,
      state: DfuState::idle(),
    }
  }
}

impl<B: UsbBus> UsbClass<B> for DfuClass {
  fn poll(&mut self) {}

  fn reset(&mut self) {
    info!("DfuDevice::reset");
  }

  #[rustfmt::skip]
  fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> usb_device::Result<()> {
    debug!("DfuClass::get_configuration_descriptors");
    writer.interface(
      self.interface,
      0xFE, // bInterfaceClass (Application specific)
      0x01, // bInterfaceSubClass (DFU)
      0x02, // bInterfaceProtocol (DFU mode)
    )?;

    let mut attributes = 0;
    if self.support_download {
      attributes |= 1;
    }
    if self.support_upload {
      attributes |= 2;
    }
    writer.write(
      0x21,                                   // bDescriptorType (DFU Functional)
      &[
        attributes,                           // bmAttributes
        0xff, 0xff,                           // wDetachTimeout (65535 ms)
        0x00, 0x04,                           // wTransferSize (1024 bytes)
        0x11, 0x1,                            // bcdDFUVersion (1.11)
      ],
    )?;

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

    info!("control_in");

    match req.request_type {
      RequestType::Class => {
        if let Ok(request) = DfuRequest::try_from(req.request) {
          match request {
            DfuRequest::Detach => error!("unhandled control in request: Detach"),
            DfuRequest::Download => info!("unhandled control in request: Download"),
            DfuRequest::Upload => {
              info!("DfuRequest::Upload");
            }

            DfuRequest::GetStatus => {
              info!("DfuRequest::GetStatus");
              let payload = &[
                self.status, // bStatus
                0,
                0,
                0,                       // bwPollTimeout
                self.state.byte_value(), // bState
                1,                       // iString
              ];
              xfer.accept_with(payload).ok();
            }

            DfuRequest::ClearStatus => info!("unhandled control in request: ClearStatus"),
            DfuRequest::GetState => info!("unhandled control in request: GetState"),
            DfuRequest::Abort => info!("unhandled control in request: Abort"),
          }
        } else {
          warn!("unknown Dfu control_in request: {}", req.request);
        }
      }
      RequestType::Standard => warn!("unhandled standard in request: {:?}", req),
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
      RequestType::Class => warn!("unexpected class out request: {:?}", req),
      RequestType::Vendor => warn!("unhandled vendor out request: {:?}", req),
      RequestType::Reserved => warn!("unhandled reserved out request: {:?}", req),
    }
  }
}
