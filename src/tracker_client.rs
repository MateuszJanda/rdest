use crate::Torrent;

#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {

}

impl TrackerClient {
    pub fn connect(metafile : &Torrent) -> Result<(), reqwest::Error> {
        let url = metafile.get_url();
        println!("{:?}", metafile.get_url());

        let info_hash = metafile.get_info_hash();
        let params = [
            ("info_hash", "xxx"),
            ("peer_id", "ABCDEFGHIJKLMNOPQRST")
        ];


        Ok(())
    }

    // async fn get_http() -> Result<(), reqwest::Error> {
    pub fn connect1() -> Result<(), reqwest::Error> {
        // netcat -l 127.0.0.1 8080
        // let body = reqwest::get("http://127.0.0.1:8080")
        //     .await?
        //     .text()
        //     .await?;

        let params = [("info_hash", "")];
        let client = reqwest::blocking::Client::new();
        let body = client
            .get("http://127.0.0.1:8080")
            .form(&params)
            .send()?
            .text();
        println!("body = {:?}", body);

        Ok(())
    }
}