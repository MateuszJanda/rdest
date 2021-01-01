# rdest
rdest is simple BitTorrent client, currently supporting [BEP3](https://www.bittorrent.org/beps/bep_0003.html#bencoding) specification.

# Examples
Running rdest from command line.
```bash
rdest get ubuntu-20.04.1-desktop-amd64.iso.torrent
```
Running rdest code.
```rust
use rdest::{Metainfo, Session};
use rdest::peer_id;
use std::path::Path;

#[tokio::main]
async fn main() {
    let path = Path::new("ubuntu-20.04.1-desktop-amd64.iso.torrent");
    let torrent_file = Metainfo::from_file(path).unwrap();

    let mut session = Session::new(torrent_file, peer_id::generate());
    session.run().await;
}
```

# References
- https://www.bittorrent.org/beps/bep_0003.html
- https://wiki.theory.org/BitTorrent_Tracker_Protocol
- https://wiki.theory.org/BitTorrentSpecification