use crate::identity::Identity;

mod identity;

#[tokio::main]
async fn main() {
	let _identity = Identity::current();
	println!("I'm alive!");
}