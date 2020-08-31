use rdest::BValue;
use rdest::Torrent;

fn main() {
    println!("Hello, world!");
    // BValue::parse(b"i4e").unwrap();
    let t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
    println!("{:?}", t);
}
