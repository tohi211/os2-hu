use std::thread;
use spsc::*;

fn main() {
	let (px, cx) = channel();
	
	thread::spawn(move || {
		px.send("Ping").unwrap();
		px.send("Ping").unwrap();
	});
	
	println!("recv: {}", cx.recv().unwrap());
	println!("recv: {}", cx.recv().unwrap());
	assert!(cx.recv().is_err());
}
