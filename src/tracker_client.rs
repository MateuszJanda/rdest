use crate::commands::TrackerCmd;
use crate::constants::{PEER_ID_SIZE, PORT};
use crate::{Metainfo, TrackerResp};
use reqwest::Response;
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::Duration;
use url::form_urlencoded;

const DELAY_MS: u64 = 1000;

/// Tracker client.
#[derive(Clone, Debug)]
pub struct TrackerClient {
    own_id: [u8; PEER_ID_SIZE],
    metainfo: Metainfo,
    tracker_ch: mpsc::Sender<TrackerCmd>,
}

impl TrackerClient {
    /// Create new tracker client
    pub fn new(
        own_id: &[u8; PEER_ID_SIZE],
        metainfo: Metainfo,
        tracker_ch: mpsc::Sender<TrackerCmd>,
    ) -> TrackerClient {
        TrackerClient {
            own_id: *own_id,
            metainfo,
            tracker_ch,
        }
    }

    /// Connect to tracker and wait for response.
    ///
    /// If tracker respond with failure caller is informed and new connection is made after 0.5 sec.
    pub async fn run(&mut self) {
        let params = [
            ("peer_id", String::from_utf8(self.own_id.to_vec()).unwrap()),
            ("port", PORT.to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("left", self.metainfo.total_length().to_string()),
            ("event", "started".to_string()),
            ("numwant", "20".to_string()),
        ];

        let client = reqwest::Client::new();
        let url = &Self::create_url(&self.metainfo);

        loop {
            match Self::parse_resp(client.get(url).query(&params).send().await).await {
                Ok(resp) => {
                    self.send_cmd(TrackerCmd::TrackerResp(resp)).await;
                    break;
                }
                Err(e) => {
                    self.send_cmd(TrackerCmd::Fail(e)).await;
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

    async fn send_cmd(&mut self, cmd: TrackerCmd) {
        self.tracker_ch
            .send(cmd)
            .await
            .expect("Can't communicate to manager");
    }

    fn create_url(metainfo: &Metainfo) -> String {
        let info_hash: String = form_urlencoded::byte_serialize(metainfo.info_hash()).collect();
        metainfo.tracker_url().clone() + "?info_hash=" + info_hash.as_str()
    }
}
