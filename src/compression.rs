use nanoserde::{DeBin, SerBin};
use std::marker::PhantomData;

#[derive(SerBin, DeBin)]
pub struct Compressed<T> {
    data: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T> Compressed<T> {
    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
    pub fn from_data(data: Vec<u8>) -> Self {
        Compressed {
            data,
            phantom: PhantomData,
        }
    }
}

pub trait Compressible: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, String>;
    fn compress(&self) -> Compressed<Self> {
        let mut encoder = snap::write::FrameEncoder::new(Vec::new());
        let input = self.to_bytes();
        std::io::copy(&mut input.as_slice(), &mut encoder).expect("");
        let data = encoder.into_inner().expect("");
        Compressed {
            data,
            phantom: PhantomData,
        }
    }
}

impl<T: SerBin + DeBin> Compressible for T {
    fn to_bytes(&self) -> Vec<u8> {
        let mut s = Vec::new();
        self.ser_bin(&mut s);
        s
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::de_bin(&mut 0, bytes).map_err(|e| e.to_string())
    }
}

impl<T: Compressible> Compressed<T> {
    pub fn decompress(&self) -> Result<T, String> {
        let mut decoder = snap::read::FrameDecoder::new(&self.data[..]);
        let mut output = Vec::new();
        std::io::copy(&mut decoder, &mut output).map_err(|e| e.to_string())?;
        T::from_bytes(&output)
    }
}
