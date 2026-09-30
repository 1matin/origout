use ed25519_dalek::SigningKey;
use getrandom::SysRng;
use getrandom::rand_core::UnwrapErr;
use std::path::Path;

#[doc = env!("SCHEMAS_HASH")]
cme::cme_schema_setup! {
    schema = "schemas/origout.cm",
    program = mod "checkmate",
    limits = { max_call_depth: 1024 },
    proxy = OrigoutIdentityProxy as identity,
    provider = fs => FsService,
    provider = ed25519 => Ed25519Service,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = Host::new()?;

    let keypair = host.try_run(|session| session.identity.get_current_identity())??;

    println!("Current public key: {}", keypair.public_key);
    Ok(())
}

struct FsService;

struct Ed25519Service;

impl origout::OrigoutEd25519Capability for Ed25519Service {
    fn generate_key_pair(&self) -> origout::KeyPair {
        let mut rng = UnwrapErr(SysRng);
        key_pair(&SigningKey::generate(&mut rng))
    }

    fn from_private_key(&self, private_key: Vec<u8>) -> origout::KeyPair {
        let bytes: [u8; 32] = private_key
            .try_into()
            .expect("Ed25519 private key must be 32 bytes");
        key_pair(&SigningKey::from_bytes(&bytes))
    }

    fn decode_private_key(&self, private_key: String) -> Vec<u8> {
        assert_eq!(
            private_key.len(),
            64,
            "Ed25519 private key must be 64 hex digits"
        );
        private_key
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("private key must be hex");
                u8::from_str_radix(pair, 16).expect("private key must be hex")
            })
            .collect()
    }
}

fn key_pair(key: &SigningKey) -> origout::KeyPair {
    origout::KeyPair {
        public_key: hex(key.verifying_key().as_bytes()),
        private_key: hex(key.as_bytes()),
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

impl origout::OrigoutFsCapability for FsService {
    fn get_config_dir(&self) -> String {
        dirs::config_dir().unwrap().to_string_lossy().into_owned()
    }

    fn path_exists(&self, path: String) -> bool {
        Path::exists(path.as_ref())
    }

    fn read_file(&self, path: String) -> String {
        String::from_utf8(std::fs::read(path).unwrap()).unwrap()
    }

    fn read_bytes(&self, path: String) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }

    fn delete_file(&self, path: String) -> bool {
        std::fs::remove_file(path).is_ok()
    }

    fn write_file(&self, path: String, content: String) -> bool {
        std::fs::write(path, content).is_ok()
    }

    fn write_bytes(&self, path: String, content: Vec<u8>) {
        std::fs::write(path, content).expect("failed to save the identity key");
    }
}
