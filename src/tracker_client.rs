#[derive(PartialEq, Clone, Debug)]
pub struct TrackerClient {

}

impl TrackerClient {
    // async fn get_http() -> Result<(), reqwest::Error> {
    pub fn connect() -> Result<(), reqwest::Error> {
        // netcat -l 127.0.0.1 8080
        // let body = reqwest::get("http://127.0.0.1:8080")
        //     .await?
        //     .text()
        //     .await?;

        let body = reqwest::blocking::get("http://127.0.0.1:8080")?.text();
        println!("body = {:?}", body);

        Ok(())
    }
}