use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::SeqCst;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AuthStateType {
  Waiting = 0,
  ReceivingNonce = 1,
  Signing = 2,
  SendingSignature = 3,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct AuthState {
  state: AuthStateType,
  nonce_id: u8,
  next_part: u8,
  padding: u8,
}

impl AuthState {
  fn from_u32(value: u32) -> AuthState {
    unsafe { core::mem::transmute_copy(&value) }
  }

  fn to_u32(self) -> u32 {
    unsafe { core::mem::transmute_copy(&self) }
  }
}

static STATE: AtomicU32 = AtomicU32::new(0);
static mut DATA: [u8; 1064] = [0; 1064];
static mut KEYPAIR: Option<ds4auth::DS4Key> = None;

fn crc(data: &[u8]) -> u32 {
  crc::crc32::checksum_ieee(data)
}

pub fn read_keypair() {
  unsafe {
    KEYPAIR = ds4auth::DS4Key::embedded();
  }
}

pub fn set_nonce(bytes: &[u8]) -> Result<(), ()> {
  if bytes.len() != 64 {
    error!("received nonce packet of incorrect length");
    return Err(());
  }

  let received_crc = &bytes[(bytes.len() - 4)..];
  let calculated_crc = crc(&bytes[..(bytes.len() - 4)]).to_le_bytes();

  if received_crc != calculated_crc {
    error!("CRC mismatch for nonce packet: {:?}", bytes);
    return Err(());
  }

  let received_nonce_id = bytes[1];
  let received_nonce_part = bytes[2];
  info!(
    "received data for nonce {}, part {}/5",
    received_nonce_id,
    received_nonce_part + 1
  );

  let state = AuthState::from_u32(STATE.load(SeqCst));
  match state.state {
    AuthStateType::Waiting => {
      if received_nonce_part != 0 {
        error!("received non-zero nonce part first?");
        return Err(());
      }
    }

    AuthStateType::ReceivingNonce => {
      if received_nonce_id != state.nonce_id {
        error!(
          "received wrong nonce id (expected {}, got {})",
          state.nonce_id, received_nonce_id
        );
        return Err(());
      }

      if received_nonce_part != state.next_part {
        error!(
          "received wrong nonce part (expected {}, got {})",
          state.next_part, received_nonce_part
        );
      }
    }

    _ => {
      error!("received nonce while in unexpected state: {:?}", state);
      return Err(());
    }
  }

  let last_packet = received_nonce_part == 4;
  let nonce_start = (56 * received_nonce_part) as usize;
  let nonce_len = if last_packet { 32 } else { 56 };
  let nonce_data = &bytes[4..];
  unsafe { DATA[nonce_start..nonce_start + nonce_len].copy_from_slice(&nonce_data[..nonce_len]) }

  if last_packet {
    info!("done receiving nonce, transitioning to signing state");
    STATE.store(
      AuthState {
        state: AuthStateType::Signing,
        nonce_id: received_nonce_id,
        next_part: 0,
        padding: 0,
      }
      .to_u32(),
      SeqCst,
    );
  } else {
    STATE.store(
      AuthState {
        state: AuthStateType::ReceivingNonce,
        nonce_id: received_nonce_id,
        next_part: received_nonce_part + 1,
        padding: 0,
      }
      .to_u32(),
      SeqCst,
    );
  }

  Ok(())
}

pub fn signature_ready() -> bool {
  AuthState::from_u32(STATE.load(SeqCst)).state == AuthStateType::SendingSignature
}

pub fn get_nonce_id() -> u8 {
  AuthState::from_u32(STATE.load(SeqCst)).nonce_id
}

pub fn get_signature_chunk(buf: &mut [u8]) -> Result<(), ()> {
  let state = AuthState::from_u32(STATE.load(SeqCst));
  if state.state != AuthStateType::SendingSignature {
    error!("received requests for signature when not sending signature");
    return Err(());
  }

  let nonce_id = state.nonce_id;
  let part = state.next_part;
  let done = part == 18;
  let offset = part as usize * 56;
  let data = unsafe { &DATA[offset..offset + 56] };

  let next_state = if done {
    0
  } else {
    AuthState {
      state: AuthStateType::SendingSignature,
      nonce_id,
      next_part: part + 1,
      padding: 0,
    }
    .to_u32()
  };

  STATE.store(next_state, SeqCst);
  buf[0] = 0xf1;
  buf[1] = nonce_id;
  buf[2] = part;
  buf[3] = 0;
  buf[4..60].copy_from_slice(data);
  let crc_bytes = crc(&buf[..60]).to_le_bytes();
  buf[60..].copy_from_slice(&crc_bytes);
  info!("sending part {}/19 of nonce {}", part + 1, nonce_id);
  Ok(())
}

pub fn perform_work() -> ! {
  loop {
    let state = AuthState::from_u32(STATE.load(SeqCst));
    if state.state == AuthStateType::Signing {
      info!("starting to sign nonce");

      let mut nonce = [0u8; 256];
      unsafe {
        nonce.copy_from_slice(&DATA[0..256]);
      }
      if let Some(signature) = unsafe { KEYPAIR.as_ref().and_then(|kp| kp.sign(&nonce)) } {
        unsafe {
          DATA[..].copy_from_slice(signature.as_bytes());
        }
      } else {
        error!("failed to sign nonce");
        break;
      }

      info!("done signing nonce");
      crate::allocator::dump_state();

      STATE.store(
        AuthState {
          state: AuthStateType::SendingSignature,
          nonce_id: state.nonce_id,
          next_part: 0,
          padding: 0,
        }
        .to_u32(),
        SeqCst,
      );
    }
  }

  loop {
    unsafe { asm!("nop") };
  }
}
