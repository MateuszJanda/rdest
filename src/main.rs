use rdest::{Manager, Metainfo};
use structopt::StructOpt;
use tokio;

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    path: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
    println!("Rdest");

    let args = Cli::from_args();
    let path = args.path;

    let metainfo = match Metainfo::from_file(path) {
        Ok(metainfo) => metainfo,
        Err(e) => panic!("[-] Can't read metafile. Error: {}", e),
    };
    let mut manager = Manager::new(metainfo, *b"ABCDEFGHIJKLMNOPQRST");
    manager.run().await;

    println!("-==[ koniec ]==-");
}
