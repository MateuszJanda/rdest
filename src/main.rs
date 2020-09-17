use rdest::BValue;
use rdest::Torrent;
use reqwest::blocking;

fn main() {
    println!("Hello, world!");
    // BValue::parse(b"i4e").unwrap();
    let t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
    // println!("{:?}", t);

    // get_http().await;
    get_http();
}

// async fn get_http() -> Result<(), reqwest::Error> {
fn get_http() -> Result<(), reqwest::Error> {

    // netcat -l 127.0.0.1 8080
    // let body = reqwest::get("http://127.0.0.1:8080")
    //     .await?
    //     .text()
    //     .await?;

    let body = reqwest::blocking::get("http://127.0.0.1:8080")?.text();
    println!("body = {:?}", body);

    Ok(())
}
