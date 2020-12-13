/// Finder interface for dictionary key in [bencoded](https://en.wikipedia.org/wiki/Bencode) string.
pub trait RawFinder {
    /// Find first value for defined key in [bencoded](https://en.wikipedia.org/wiki/Bencode) string with dictionaries.
    fn find_first(key: &str, arg: &[u8]) -> Option<Vec<u8>>;
}
