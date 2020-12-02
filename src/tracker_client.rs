use crate::constant::{HASH_SIZE, PORT};
use crate::{Metainfo, TrackerResp};
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::Response;
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::Duration;
use url::form_urlencoded;

const DELAY_MS: u64 = 500;

#[derive(Debug, Clone)]
pub enum TrackerCmd {
    TrackerResp(TrackerResp),
    Fail(String),
}

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {}

impl TrackerClient {
    pub async fn run(
        peer_id: &[u8; HASH_SIZE],
        metainfo: &Metainfo,
        tracker_ch: &mut mpsc::Sender<TrackerCmd>,
    ) {
        let params = [
            ("peer_id", String::from_utf8(peer_id.to_vec()).unwrap()),
            ("port", PORT.to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", metainfo.total_length().to_string()),
            ("event", "started".to_string()),
            ("numwant", "20".to_string()),
        ];

        let client = reqwest::Client::new();
        let url = &Self::get_url(metainfo);

        loop {
            match Self::parse_resp(client.get(url).query(&params).send().await).await {
                Ok(resp) => {
                    Self::send(tracker_ch, TrackerCmd::TrackerResp(resp)).await;
                    break;
                }
                Err(e) => {
                    Self::send(tracker_ch, TrackerCmd::Fail(e)).await;
                    time::delay_for(Duration::from_millis(DELAY_MS)).await;
                }
            }
        }
    }

    async fn parse_resp(resp: Result<Response, reqwest::Error>) -> Result<TrackerResp, String> {
        match resp {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Err(resp.status().to_string());
                }

                match resp.bytes().await {
                    Ok(body) => match TrackerResp::from_bencode(body.as_ref()) {
                        Ok(data) => Ok(data),
                        Err(e) => Err(e.to_string()),
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    async fn send(tracker_ch: &mut mpsc::Sender<TrackerCmd>, cmd: TrackerCmd) {
        tracker_ch
            .send(cmd)
            .await
            .expect("Can't communicate to manager");
    }

    pub async fn connect(metainfo: &Metainfo) -> Result<TrackerResp, Box<dyn std::error::Error>> {
        let peer_id = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(HASH_SIZE)
            .collect::<String>();

        let params = [
            ("peer_id", peer_id),
            ("port", PORT.to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", metainfo.total_length().to_string()),
            ("event", "started".to_string()),
            ("numwant", "20".to_string()),
        ];

        let client = reqwest::Client::new();
        let resp = client
            .get(&Self::get_url(metainfo))
            .query(&params)
            .send()
            .await?;

        let body = resp.bytes().await?;
        let data = match TrackerResp::from_bencode(body.as_ref()) {
            Ok(data) => data,
            Err(e) => Err(e)?,
        };

        Ok(data)
    }

    fn get_url(metainfo: &Metainfo) -> String {
        let info_hash: String = form_urlencoded::byte_serialize(metainfo.info_hash()).collect();
        metainfo.tracker_url().clone() + "?info_hash=" + info_hash.as_str()
    }
}
