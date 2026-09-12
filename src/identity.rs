use ed25519_dalek::SigningKey;
use getrandom::SysRng;
use getrandom::rand_core::UnwrapErr;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct Identity {
	keypair: SigningKey,
}

impl Identity {
	pub fn current() -> Identity {
		let mut key_path = dirs::config_dir().unwrap();
		key_path.push("origout.conf");

		let keypair: SigningKey;

		if !Path::exists(&key_path) {
			let mut csprng = UnwrapErr(SysRng);
			keypair = SigningKey::generate(&mut csprng);

			let mut key_file = File::create(key_path).unwrap();
			key_file.write_all(keypair.as_bytes()).unwrap();
		} else {
			let raw_keypair = std::fs::read(key_path).unwrap();
			let raw_keypair: &[u8; 32] = raw_keypair
				.as_slice()
				.try_into()
				.unwrap();

			keypair = SigningKey::from_bytes(raw_keypair);
		}

		Identity { keypair }
	}
}