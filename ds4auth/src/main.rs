fn main() {
  let keypair = ds4auth::DS4Key::embedded().unwrap();
  let nonce = [0u8; 256];
  let signature = keypair.sign(&nonce).unwrap();
  assert!(signature.validate(&nonce));
}
