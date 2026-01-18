use std::io::{Result, Write};

pub struct LazyWriter<W: Write> {
    writer: W,
    header: String,
    has_printed_header: bool,
}

impl<W: Write> LazyWriter<W> {
    pub fn new(writer: W, header: String) -> Self {
        Self {
            writer: writer,
            header: header,
            has_printed_header: false,
        }
    }
}

impl<W: Write> Write for LazyWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if !self.has_printed_header {
            writeln!(self.writer, "{}", self.header)?;
            self.has_printed_header = true;
        }

        self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
