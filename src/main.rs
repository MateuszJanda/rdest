use rdest::peer_id;
use rdest::{Manager, Metainfo};
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
#[structopt(
    name = "rdest",
    author = "Mateusz Janda <mateusz.janda@gmail.com>",
    about = "A simple BitTorrent Optent"
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "path to .torrent file")]
    path: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Opt::from_args();
    let path = args.path;

    let metainfo = match Metainfo::from_file(path) {
        Ok(metainfo) => metainfo,
        Err(e) => panic!("[-] Can't read metafile. Error: {}", e),
    };
    let own_id = peer_id::generate();
    let mut manager = Manager::new(metainfo, own_id);
    manager.run().await;

    println!("-==[ koniec ]==-");
}
