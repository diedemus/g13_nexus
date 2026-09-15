fn main() {
    println!("G13 Daemon running...");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
