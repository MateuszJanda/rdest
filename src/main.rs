// use rdest::BValue;
use rdest::{Torrent, ResponseParser};
// use rdest::TrackerClient;
// use hex_literal::hex;


fn main() {
    println!("Hello, world!");
    // let b = BValue::parse(b"i4e").unwrap();
    let _t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
    // println!("{:?}", t);

    // match TrackerClient::connect1(&t.unwrap()) {
    //     Ok(_) => println!("Http Ok"),
    //     Err(e) => println!("Http Problem {:?}", e),
    // }

    println!("{:?}", ResponseParser::from_file("response.data".to_string()));
}

