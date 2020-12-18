use rdest::peer_id;
use rdest::{Metainfo, PeerManager};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
#[structopt(
    name = "rdest",
    author = "Mateusz Janda <mateusz.janda@gmail.com>",
    about = "A simple BitTorrent client"
)]
enum Opt {
    /// Get torrent files from p2p network
    Get(Get),
    /// Create .torrent file
    Create(Create),
}

#[derive(StructOpt)]
struct Get {
    /// Path to .torrent file
    #[structopt(parse(from_os_str), name = "PATH")]
    path: PathBuf,
}

#[derive(StructOpt)]
struct Create {
    /// Create .torrent for file
    #[structopt(parse(from_os_str), short = "-c", long = "--create", name = "FILE")]
    path: PathBuf,
    /// Tracker address
    #[structopt(short, long, name = "ADDRESS")]
    tracker_addr: String,
}

#[tokio::main]
async fn main() {
    match Opt::from_args() {
        Opt::Get(get) => get_torrent(&get.path).await,
        Opt::Create(create) => create_torrent(&create.path, &create.tracker_addr).await,
    };
}

async fn get_torrent(path: &PathBuf) {
    let metainfo = match Metainfo::from_file(path.as_path()) {
        Ok(metainfo) => metainfo,
        Err(e) => panic!("[-] Can't read metafile. Error: {}", e),
    };
    let mut manager = PeerManager::new(metainfo, peer_id::generate());
    manager.run().await;

    println!("-==[ koniec ]==-");
}

async fn create_torrent(path: &PathBuf, tracker_addr: &String) {
    match Metainfo::create_file(path.as_path(), tracker_addr) {
        Ok(()) => (),
        Err(e) => panic!("[-] Can't create metafile. Error: {}", e),
    }
}
