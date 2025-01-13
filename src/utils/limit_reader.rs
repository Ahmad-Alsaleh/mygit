use std::io;

pub(crate) struct LimitReader<R: io::Read> {
    reader: R,
    limit: usize,
}

impl<R: io::Read> LimitReader<R> {
    pub(crate) fn new(reader: R, limit: usize) -> Self {
        Self { reader, limit }
    }
}

impl<R: io::Read> io::Read for LimitReader<R> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() > self.limit {
            buf = &mut buf[..self.limit + 1];
        }

        let n = self.reader.read(buf)?;
        if n > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "LimitReader read more than sepcified limit of {} bytes (read {} bytes)",
                    self.limit, n
                ),
            ));
        }

        self.limit -= n;
        Ok(n)
    }
}
