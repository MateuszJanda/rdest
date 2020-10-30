use rdest::{
    Bitfield, Command, Connection, Error, Frame, Handler, Handshake, Metainfo, Request,
    TrackerClient,
};
use std::net::Ipv4Addr;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

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

    let (mut tx, mut rx) = mpsc::channel(32);

    let pieces_len = t.pieces().len();
    let piece_length = t.piece_length();
    let mut peer_bitfield = vec![false; t.pieces().len()];

    let manager = tokio::spawn(async move {
        let my_pieces = vec![false; pieces_len];
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::RecvBitfield {
                    key,
                    bitfield,
                    channel,
                } => {
                    peer_bitfield = bitfield.available_pieces();

                    for i in 0..my_pieces.len() {
                        if my_pieces[i] == false && peer_bitfield[i] == true {
                            let my = Bitfield::new(vec![0; pieces_len]);
                            channel.send(Command::SendBitfield {
                                bitfield: my,
                                interested: true,
                            });
                            break;
                        }
                    }
                }
                Command::RecvUnchoke { key, channel } => {
                    for i in 0..my_pieces.len() {
                        if my_pieces[i] == false && peer_bitfield[i] == true {
                            let my = Request::new(i, 0, piece_length as usize);
                            channel.send(Command::SendRequest { req: my });
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
    });

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

    println!("-==[ koniec ]==-");
}
