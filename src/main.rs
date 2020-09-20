// use rdest::BValue;
use rdest::Torrent;
use rdest::TrackerClient;

fn main() {
    println!("Hello, world!");
    // let b = BValue::parse(b"i4e").unwrap();
    let t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
    // println!("{:?}", t);

    // get_http().await;
    match TrackerClient::connect(&t.unwrap()) {
        Ok(_) => println!("Http Ok"),
        _ => println!("Http Problem"),
    }
}
