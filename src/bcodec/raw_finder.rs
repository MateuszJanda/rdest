/// Interface for finder in [bencoded](https://en.wikipedia.org/wiki/Bencode) dictionaries.
pub trait RawFinder {
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>>;
}
