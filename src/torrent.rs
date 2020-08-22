

pub struct Torrent {
    announce : String,
    info : Info,

}

pub struct Info {
    name : String,
    piece_length : i32,
    pieces : String,
    length : Option(i32),
    files : Option(Vec<File>),
}

pub struct File {
    length : i32,
    path : Vec<String>,
}