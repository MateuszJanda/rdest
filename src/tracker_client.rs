extern crate rand;

extern crate url;
use url::form_urlencoded;


use crate::Torrent;
use rand::Rng;
use rand::distributions::Alphanumeric;

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {}

impl TrackerClient {
    pub fn connect(metafile: &Torrent) -> Result<(), reqwest::Error> {
        let url = metafile.url();
        println!("{:?}", metafile.url());

        // let info_hash = metafile.info_hash();
        let params = [("info_hash", "xxx"), ("peer_id", "ABCDEFGHIJKLMNOPQRST")];

        Ok(())
    }

    // async fn get_http() -> Result<(), reqwest::Error> {
    pub fn connect1(metafile: &Torrent) -> Result<(), reqwest::Error> {
        // netcat -l 127.0.0.1 8080
        // let body = reqwest::get("http://127.0.0.1:8080")
        //     .await?
        //     .text()
        //     .await?;

        // let u = "http://127.0.0.1:8080/".to_string();
        let u = metafile.url();
        let hash : String = form_urlencoded::byte_serialize(&metafile.hash).collect();
        let url = u + "?info_hash=" + hash.as_str();

        println!("url = {:?}", url);
        println!("hash = {:?}", metafile.hash);

        let peer_id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .collect::<String>();

        println!("{:?}", peer_id);

        let params = [
            ("peer_id", peer_id),
            ("port", "6882".to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", metafile.length().to_string()),
            ("event", "started".to_string()),
            ("numwant", "50".to_string()),
        ];

        let client = reqwest::blocking::Client::new();
        let body = client
            .get(&url)
            .query(&params)
            .send()?
            .text();
        println!("body = {:?}", body);

        Ok(())
    }
}
