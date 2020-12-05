pub trait Serializer {
    fn data(&self) -> Vec<u8>;
}
