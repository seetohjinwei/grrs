use std::io::{Result, Stdout, Write};

const BUF_SIZE: usize = 8192;

pub struct SynchronizedWriter {
    writer: Stdout,
    header: String,
    buf: Vec<u8>,
}

impl SynchronizedWriter {
    pub fn new(writer: Stdout, header: String) -> Self {
        Self {
            writer: writer,
            header: header,
            buf: Vec::with_capacity(BUF_SIZE),
        }
    }
}

impl Write for SynchronizedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.buf.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        if self.buf.len() == 0 {
            return Ok(());
        }

        let mut writer = self.writer.lock();

        writer.write_fmt(format_args!("{}\n", self.header))?;
        writer.write_all(&self.buf)?;

        let result = writer.flush()?;

        self.buf.clear();

        Ok(result)
    }
}

impl Drop for SynchronizedWriter {
    fn drop(&mut self) {
        // TODO: Do we need to handle this failing?
        let _ = self.flush();
    }
}
