#![no_std]

#[macro_use]
extern crate log;

use core::fmt;
use ring::signature::{KeyPair, RsaKeyPair, RsaPublicKeyComponents};

#[repr(packed)]
pub struct DS4Signature {
  pub nonce_sig: [u8; 256],
  pub serial: [u8; 16],
  pub n: [u8; 256],
  pub e: [u8; 256],
  pub key_sig: [u8; 256],
  pub padding: [u8; 24],
}

fn trim(mut slice: &[u8]) -> &[u8] {
  while let Some(0) = slice.first() {
    slice = &slice[1..];
  }
  slice
}

impl DS4Signature {
  pub fn parse(bytes: [u8; 1064]) -> DS4Signature {
    let mut result = DS4Signature {
      nonce_sig: [0; 256],
      serial: [0; 16],
      n: [0; 256],
      e: [0; 256],
      key_sig: [0; 256],
      padding: [0; 24],
    };
    result.nonce_sig.copy_from_slice(&bytes[0..256]);
    result.serial.copy_from_slice(&bytes[256..272]);
    result.n.copy_from_slice(&bytes[272..528]);
    result.e.copy_from_slice(&bytes[528..784]);
    result.key_sig.copy_from_slice(&bytes[784..1040]);
    result.padding.copy_from_slice(&bytes[1040..1064]);
    result
  }

  pub fn public_key(&self) -> RsaPublicKeyComponents<&[u8]> {
    let n = trim(self.n.as_ref());
    let e = trim(self.e.as_ref());
    RsaPublicKeyComponents { n, e }
  }

  pub fn validate(&self, nonce: &[u8]) -> bool {
    self
      .public_key()
      .verify(
        &ring::signature::RSA_PSS_2048_8192_SHA256,
        nonce,
        self.nonce_sig.as_ref(),
      )
      .is_ok()
  }

  pub fn as_bytes(&self) -> &[u8] {
    unsafe {
      core::slice::from_raw_parts(
        (self as *const DS4Signature) as *const u8,
        core::mem::size_of::<DS4Signature>(),
      )
    }
  }
}

impl fmt::Debug for DS4Signature {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "DS4Signature {{\n\tnonce signature = {:x?}\n\tserial = {:x?}\n\tn = {:x?}\n\te = {:x?}\n\tkey signature = {:x?}\n}}",
      self.nonce_sig.as_ref(),
      self.serial.as_ref(),
      self.n.as_ref(),
      self.e.as_ref(),
      self.key_sig.as_ref()
    )
  }
}

fn zero_extend_256(value: ring::io::Positive) -> Option<[u8; 256]> {
  let mut buf = [0u8; 256];
  let bytes = value.big_endian_without_leading_zero();
  if bytes.len() > buf.len() {
    None
  } else {
    let start = buf.len() - bytes.len();
    &buf[start..].copy_from_slice(bytes);
    Some(buf)
  }
}

pub struct DS4Key {
  serial: &'static [u8; 16],
  keypair: RsaKeyPair,
  signature: &'static [u8; 256],
}

impl DS4Key {
  pub fn embedded() -> Option<DS4Key> {
    if let Ok(keypair) = RsaKeyPair::from_der(include_bytes!("../../keys/ds4.der")) {
      let serial = include_bytes!("../../keys/ds4.serial");
      let signature = include_bytes!("../../keys/ds4.sig");
      Some(DS4Key {
        serial: serial,
        keypair,
        signature: signature,
      })
    } else {
      None
    }
  }

  pub fn sign(&self, nonce: &[u8]) -> Option<DS4Signature> {
    let mut signature = [0; 256];
    let rand = ring::rand::SystemRandom::new();
    if let Err(e) = self
      .keypair
      .sign(&ring::signature::RSA_PSS_SHA256, &rand, nonce, &mut signature)
    {
      error!("failed to sign: {}", e);
      return None;
    }

    Some(DS4Signature {
      nonce_sig: signature,
      serial: self.serial.clone(),
      n: zero_extend_256(self.keypair.public_key().modulus()).unwrap(),
      e: zero_extend_256(self.keypair.public_key().exponent()).unwrap(),
      key_sig: self.signature.clone(),
      padding: [0u8; 24],
    })
  }
}

#[repr(packed)]
/// Format of the DS4 key on flash.
pub struct DS4KeyEncoded {
  pub serial: [u8; 16],
  pub n: [u8; 256],
  pub e: [u8; 256],

  /// Signature of SHA256(serial | n | e)
  pub sig: [u8; 256],

  pub p: [u8; 128],
  pub q: [u8; 128],
  pub dp: [u8; 128],
  pub dq: [u8; 128],
  pub qinv: [u8; 128],
}
