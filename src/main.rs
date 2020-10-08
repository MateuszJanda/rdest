// use rdest::BValue;
use rdest::{Torrent, ResponseParser};
// use rdest::TrackerClient;
// use hex_literal::hex;
use tokio;
use tokio::net::{TcpListener, TcpStream};


// fn main() {
//     println!("Hello, world!");
//     // let b = BValue::parse(b"i4e").unwrap();
//     let _t = Torrent::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent"));
//     // println!("{:?}", t);
//
//     // match TrackerClient::connect1(&t.unwrap()) {
//     //     Ok(_) => println!("Http Ok"),
//     //     Err(e) => println!("Http Problem {:?}", e),
//     // }
//
//     println!("{:?}", ResponseParser::from_file("response.data".to_string()));
// }


#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let mut listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await.unwrap();
        println!("accept");
        // process(socket).await;
    }

}


/*
async fn process(socket: TcpStream) {
    // The `Connection` lets us read/write redis **frames** instead of
    // byte streams. The `Connection` type is defined by mini-redis.
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        // Respond with an error
        let response = Frame::Error("unimplemented".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}

 */
