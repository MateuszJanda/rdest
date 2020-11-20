use crate::constant::HASH_SIZE;
use crate::{Metainfo, TrackerResp};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::fs;
use url::form_urlencoded;

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {}

impl TrackerClient {
    pub async fn connect(metafile: &Metainfo) -> Result<TrackerResp, Box<dyn std::error::Error>> {
        let peer_id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(HASH_SIZE)
            .collect::<String>();

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
        let resp = client
            .get(&Self::get_url(metafile))
            .query(&params)
            .send()
            .await?;
        let body = resp.bytes().await?;

        fs::write("response.data", &body).unwrap(); // TODO: remove

        let data = TrackerResp::from_bencode(body.as_ref())?;
        Ok(data)
    }

    fn get_url(metafile: &Metainfo) -> String {
        let info_hash: String = form_urlencoded::byte_serialize(metafile.info_hash()).collect();
        metafile.tracker_url().clone() + "?info_hash=" + info_hash.as_str()
    }
}
