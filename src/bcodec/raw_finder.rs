/// Interface for finder in [bencoded](https://en.wikipedia.org/wiki/Bencode) dictionaries.
pub trait RawFinder {
    /// Find first value for defined key in [bencoded](https://en.wikipedia.org/wiki/Bencode) string.
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>>;
}
