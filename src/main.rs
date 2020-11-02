use rdest::{Handler, Manager, Metainfo, TrackerClient};
use std::net::Ipv4Addr;
use tokio;
use tokio::net::TcpListener;
// use tokio::sync::mpsc;

/*
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
enum Command {
    Rrr {
        key: String,
        resp: oneshot::Sender<Command>,
    },
    Sss {
        key: String,
    },
}

struct Mmm {
    rx: mpsc::Receiver<Command>,
}

impl Mmm {
    async fn run(&mut self) {
        if let Some(Command::Rrr{key, resp}) = self.rx.recv().await {
            println!("Manager: recv: {}", key);

            let s = Command::Sss {
                key: "jkl".to_string(),
            };

            let _ = resp.send(s);
        }
    }
}

#[tokio::main]
async fn main() {
    let (mut tx, rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let mut m = Mmm {
        rx
    };

    // let manager = tokio::spawn(async move { m.run(); });
    let manager = tokio::spawn(async move { m.run().await; });

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = Command::Rrr {
            key: "asdf".to_string(),
            resp: resp_tx,
        };

        println!("Job: sending");
        if tx.send(cmd).await.is_err() {
            eprintln!("connection task shutdown");
            return;
        }

        let res = resp_rx.await;
        println!("Job: GOT = {:?}", res);
    });

    t1.await.unwrap();
    manager.await.unwrap();

    // m.run().await;
}

 */

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // let mut listener = TcpListener::bind("127.0.0.1:6881").await.unwrap();
    let mut listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), 6881))
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


    // let manager = tokio::spawn(Manager::fff(t, r));
    // manager.await.unwrap();

    // Manager::fff(t, r).await;

    let mut m = Manager::new(t, r);

    println!("BUKA 1");
    let manager = tokio::spawn(async move { m.run().await; });
    manager.await.unwrap();

    /*
    let (mut tx, mut rx) = mpsc::channel(32);

    let pieces_len = t.pieces().len();
    println!("pieces_len {:?}", t.pieces().len());

    let piece_length = t.piece_length();

    let manager = tokio::spawn(Manager::run(pieces_len, &mut rx.clone()));

    let addr = r.peers()[2].clone();
    let info_hash = t.info_hash();
    let peer_id = b"ABCDEFGHIJKLMNOPQRST";

    let tx2 = mpsc::Sender::clone(&tx);
    // let mut tx2 = tx.clone();

    // let job = tokio::spawn(async move {
    //
    //     // let addr = "127.0.0.1:8888";
    //     println!("Try connect to {}", &addr);
    //     let socket = TcpStream::connect(&addr).await.unwrap();
    //     let connection = Connection::new(addr, socket);
    //     println!("connect");
    //
    //     let mut handler2 = Handler { connection, tx: tx2 };
    //
    //     // Process the connection. If an error is encountered, log it.
    //     if let Err(err) = handler2.run2(&info_hash, peer_id).await {
    //         // error!(cause = ?err, "connection error");
    //         panic!("jkl");
    //     }
    // });

    let job = tokio::spawn(Handler::fff(addr, info_hash, *peer_id, tx2));

    job.await.unwrap();
    manager.await.unwrap();

    // {
    //     let addr = "127.0.0.1:8888";
    //     let socket = TcpStream::connect(addr).await.unwrap();
    //     let connection = Connection::new(socket);
    //
    //     let mut handler3 = Handler { connection };
    //
    //     tokio::spawn(async move {
    //         if let Err(err) = handler3.run3().await {
    //             panic!("jkl");
    //         }
    //     });
    // }

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
*/
    println!("-==[ koniec ]==-");
}
