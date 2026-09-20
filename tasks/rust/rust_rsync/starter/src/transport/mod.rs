use std::io;

pub mod daemon;
pub mod local;
pub mod shell;

pub trait Transport {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn receive(&mut self, buffer: &mut [u8]) -> io::Result<usize>;
    fn close(&mut self) -> io::Result<()>;
}
