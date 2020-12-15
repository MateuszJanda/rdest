use rdest::peer_id;
use rdest::{Manager, Metainfo};
use structopt::StructOpt;
use tokio;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(
    name = "rdest",
    author = "Mateusz Janda <mateusz.janda@gmail.com>",
    about = "A simple BitTorrent client"
)]
enum Opt {
    /// Fetch torrent files from p2p network
    Fetch(Fetch),
    /// Create .torrent file
    Create(Create),
}

#[derive(StructOpt)]
struct Fetch {
    /// Path to .torrent file
    #[structopt(parse(from_os_str), name = "PATH")]
    path: PathBuf,
}

#[derive(StructOpt)]
struct Create {
    /// Create .torrent for file
    #[structopt(parse(from_os_str), name = "FILE")]
    create: PathBuf,
    /// Tracker address
    #[structopt(short, long, name = "ADDRESS")]
    tracker_addr: String,
}

#[tokio::main]
async fn main() {
    let path = match Opt::from_args() {
        Opt::Fetch(fetch) => fetch.path,
        Opt::Create(create) => panic!("TODO"),
    };

    let metainfo = match Metainfo::from_file(path.as_path()) {
        Ok(metainfo) => metainfo,
        Err(e) => panic!("[-] Can't read metafile. Error: {}", e),
    };
    let mut manager = Manager::new(metainfo, peer_id::generate());
    manager.run().await;

    println!("-==[ koniec ]==-");
}
