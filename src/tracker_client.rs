extern crate rand;

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

        let url = metafile.url();
        println!("url = {:?}", metafile.url());
        println!("hash = {:?}", metafile.hash);

        let peer_id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .collect::<String>();

        println!("{:?}", peer_id);

        let a: &[u8] = &[49];
        let params = [
            // ("info_hash", metafile.hash.clone()),
            // ("info_hash", "ä"),
            ("info_hash", a),
            ("peer_id", "qqqwwweee".as_bytes()),
        //     ("peer_id", peer_id),
        //     ("port", "6882".to_string()),
        //     ("uploaded", "0".to_string()),
        //     ("downloaded", "0".to_string()),
        //     ("left", metafile.length().to_string()),
        //     ("event", "started".to_string()),
        //     // ("numwant", "50".to_string()),
        //     ("numwant", "x x x".to_string()),
        ];

        let client = reqwest::blocking::Client::new();


        println!("{:?}", client
            .get("http://127.0.0.1:8080")
            // .form(&params)
            .body("a a a&xxx=1")
            .build());

        let bytes: Vec<u8> = vec![1, 10, 100];

        let body = client
            .get("http://127.0.0.1:8080")
            .form(&params)
            .body("a a a&xxx=1")
            .body(bytes)
            .send()?
            .text();
        println!("body = {:?}", body);

        Ok(())
    }
}
