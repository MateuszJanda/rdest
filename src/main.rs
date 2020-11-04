use rdest::{Manager, Metainfo, TrackerClient};
use std::net::Ipv4Addr;
use tokio;
use tokio::net::TcpListener;
use std::io;
use std::io::Write;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // ttt().await;
    // panic!("asdf");

    // let mut listener = TcpListener::bind("127.0.0.1:6881").await.unwrap();
    let mut _listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), 6881))
        .await
        .unwrap();

    let t = Metainfo::from_file(String::from("ubuntu-20.04.1-desktop-amd64.iso.torrent")).unwrap();
    // println!("{:?}", t);

    // let r = TrackerResp::from_file("response.data".to_string()).unwrap();
    let r = TrackerClient::connect(&t).await.unwrap(); // TODO

    // for v in r.peers {
    //     println!("{:?}", v);
    // }

    // println!("{:?}", ResponseParser::from_file("response.data".to_string()));

    let mut m = Manager::new(t, r, *b"ABCDEFGHIJKLMNOPQRST");

    let manager = tokio::spawn(async move {
        m.run().await;
    });
    manager.await.unwrap();

    // loop {
    //     println!("Listening");
    //     // The second item contains the IP and port of the new connection.
    //     let (socket, _) = listener.accept().await.unwrap();
    //     println!("accept");
    //
    //     let connection = Connection::new(socket);
    //
    //     let mut handler = Handler { connection };
    //
    //     tokio::spawn(async move {
    //         // Process the connection. If an error is encountered, log it.
    //         if let Err(err) = handler.run().await {
    //             // error!(cause = ?err, "connection error");
    //             panic!("asdf");
    //         }
    //     });
    // }

    println!("-==[ koniec ]==-");
}

async fn ttt() {
    use tokio::sync::oneshot;
    use tokio::time::{interval_at, Duration, Instant};

    let start = Instant::now() + Duration::from_millis(0);
    let mut interval = interval_at(start, Duration::from_millis(1000));

    let mut i = 0;
    loop {
        interval.tick().await;
        println!("\rtick {}", i);
        print!("{}", i);

        io::stdout().flush().unwrap();

        i += 1;

        if i > 5 {
            break;
        }
    }
}
