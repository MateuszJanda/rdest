use crate::{Metainfo, TrackerResp};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::fs;
use url::form_urlencoded;

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {}

impl TrackerClient {
    pub fn connect(metafile: &Metainfo) -> Result<(), reqwest::Error> {
        let _url = metafile.tracker_url();
        println!("{:?}", metafile.tracker_url());

        // let info_hash = metafile.info_hash();
        let _params = [("info_hash", "xxx"), ("peer_id", "ABCDEFGHIJKLMNOPQRST")];

        Ok(())
    }

    pub async fn connect1(metafile: &Metainfo) -> Result<TrackerResp, Box<dyn std::error::Error>> {
        let u = metafile.tracker_url();
        let info_hash: String = form_urlencoded::byte_serialize(&metafile.info_hash()).collect();
        let url = u + "?info_hash=" + info_hash.as_str();

        println!("url = {:?}", url);
        println!("info_hash = {:?}", metafile.info_hash());

        let peer_id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .collect::<String>();

        println!("{:?}", peer_id);

        let params = [
            ("peer_id", peer_id),
            ("port", "6881".to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", metafile.total_length().to_string()),
            ("event", "started".to_string()),
            ("numwant", "50".to_string()),
        ];

        let client = reqwest::Client::new();
        let resp = client.get(&url).query(&params).send().await?;
        println!("resp = {:?}", resp);
        let body = resp.bytes().await?;
        println!("body = {:?}", body);

        fs::write("response.data", &body).unwrap();

        let rrr = TrackerResp::from_bencode(body.as_ref())?;

        Ok(rrr)
    }
}
