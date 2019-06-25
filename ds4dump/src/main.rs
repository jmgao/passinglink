use std::time::Duration;

use hidapi::HidDevice;
use ring::signature::{KeyPair, RsaKeyPair};

use ds4auth::*;

fn read(device: &HidDevice, report: u8, length: usize) -> Option<Vec<u8>> {
  let mut result = vec![0; length];
  result[0] = report;
  if let Ok(bytes) = device.get_feature_report(&mut result) {
    result.resize(bytes, 0);
    Some(result)
  } else {
    None
  }
}

fn crc(data: &[u8]) -> u32 {
  crc::crc32::checksum_ieee(data)
}

fn read_with_crc(device: &HidDevice, report: u8, length: usize) -> Option<Vec<u8>> {
  let buf = read(device, report, length);
  if let Some(bytes) = buf {
    let calculated_crc = crc(&bytes[..(bytes.len() - 4)]).to_le_bytes();
    let actual_crc = &bytes[(bytes.len() - 4)..];
    if &calculated_crc != actual_crc {
      println!("warning: crc mismatch");
    }
    Some(bytes)
  } else {
    None
  }
}

fn write_with_crc(device: &HidDevice, data: &[u8]) {
  let mut buf = Vec::new();
  buf.extend_from_slice(data);
  let checksum = crc(data).to_le_bytes();
  buf.extend_from_slice(&checksum[..]);
  println!("writing: {:x?}", buf);
  device.send_feature_report(&buf).unwrap()
}

fn send_nonce(device: &HidDevice, nonce: &[u8; 256], nonce_id: u8) {
  let mut buf = [0u8; 60];
  println!("sending nonce");
  for i in 0..5 {
    buf[0] = 0xf0;
    buf[1] = nonce_id;
    buf[2] = i;

    let start = 56 * i as usize;
    let nonce_len = if i == 4 { 32 } else { 56 };
    let nonce_data = &nonce[start..start + nonce_len];

    buf[4..4 + nonce_len].copy_from_slice(nonce_data);
    println!("writing nonce {}: {}/5", nonce_id, i + 1);
    write_with_crc(device, &buf);
  }
}

fn read_signature(device: &HidDevice) -> [u8; 1064] {
  let mut signature = [0u8; 1064];
  for i in 0..19 {
    let buf = read_with_crc(device, 0xf1, 64).expect("failed to read signature");
    println!("0xf1 = {:?}", buf);
    assert_eq!(i as u8, buf[2]);
    signature[(i * 56)..((i + 1) * 56)].copy_from_slice(&buf[4..60]);
  }
  signature
}

pub fn main() {
  let hidapi = hidapi::HidApi::new().unwrap();
  let devices = [
    (0x054c, 0x05c4), // Sony DualShock 4
    (0x1532, 0x0401), // Razer Panthera
    (0x1209, 0x214d), // Passing Link
  ];

  let device = devices
    .iter()
    .find_map(|(vid, pid)| hidapi.open(*vid, *pid).ok())
    .expect("failed to open device");

  let manufacturer = device.get_manufacturer_string().unwrap().unwrap();
  let product = device.get_product_string().unwrap().unwrap();
  println!("Successfully opened {} {}", manufacturer, product);

  device.set_blocking_mode(true).expect("failed to set blocking mode");

  let nonce = [0; 256];

  let f2 = read_with_crc(&device, 0xf2, 16).expect("failed to read 0xf2");
  println!("0xf2 = {:?}", f2);

  std::thread::sleep(Duration::from_millis(1000));

  send_nonce(&device, &nonce, 1);

  loop {
    let f2 = read_with_crc(&device, 0xf2, 16).expect("failed to read 0xf2");
    println!("0xf2 = {:?}", f2);
    if f2[2] == 0 {
      break;
    }

    std::thread::sleep(Duration::from_millis(1000));
  }

  let signature = DS4Signature::parse(read_signature(&device));
  println!("received signature: {:?}", signature);
  if signature.validate(&nonce) {
    println!("valid signature received");
  } else {
    println!("error: signature invalid");
  }

  let key = std::fs::read("../keys/ds4.der").expect("failed to read key");
  let keypair = RsaKeyPair::from_der(&key).expect("failed to parse key");
  let public_key = keypair.public_key();
  let received_public_key = signature.public_key();
  assert_eq!(
    public_key.modulus().big_endian_without_leading_zero(),
    received_public_key.n
  );
  assert_eq!(
    public_key.exponent().big_endian_without_leading_zero(),
    received_public_key.e
  );
  println!("public key matches");

  let cert = std::fs::read("../keys/ds4.sig").expect("failed to read cert");
  assert_eq!(cert, signature.key_sig.as_ref());
  println!("cert matches");

  let serial = std::fs::read("../keys/ds4.serial").expect("failed to read serial");
  assert_eq!(serial, signature.serial.as_ref());
  println!("serial matches");
}
