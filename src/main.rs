use rdest::{Manager, Metainfo};
use tokio;

#[tokio::main]
async fn main() {
    println!("Rdest");

    let metainfo =
        Metainfo::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent")).unwrap();
    let mut manager = Manager::new(metainfo, *b"ABCDEFGHIJKLMNOPQRST");
    manager.run().await;

    println!("-==[ koniec ]==-");
}
