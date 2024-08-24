use std::io::ErrorKind::UnexpectedEof;
use std::io::{BufReader, Read, Write};
use std::mem::size_of;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zerocopy::{big_endian, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const GZIP_HEADER_COMMENT: &[u8] = b"taptap capture";

#[derive(Debug)]
pub struct Reader<R: Read>(BufReader<flate2::bufread::GzDecoder<BufReader<R>>>);

impl<R: Read> Reader<R> {
    pub fn new(reader: R) -> std::io::Result<Self> {
        let gz = flate2::bufread::GzDecoder::new(BufReader::new(reader));
        if let Some(h) = gz.header() {
            if h.comment() != Some(GZIP_HEADER_COMMENT) {
                // warn?
            }
        }

        Ok(Self(BufReader::new(gz)))
    }
}

impl<R: Read> Iterator for Reader<R> {
    type Item = std::io::Result<(Vec<u8>, SystemTime)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut record = [0u8; size_of::<Record>()];
        match self.0.read_exact(&mut record) {
            Err(e) if e.kind() == UnexpectedEof => {
                return None;
            }
            Err(e) => return Some(Err(e)),
            Ok(_) => {}
        };

        let record = Record::ref_from_bytes(&record).unwrap(); // infallible
        let mut data = vec![0; record.data_length.get() as usize];
        Some(match self.0.read_exact(&mut data) {
            Err(e) => Err(e),
            Ok(_) => Ok((data, record.timestamp())),
        })
    }
}

#[derive(Debug)]
pub struct Writer<W: Write>(flate2::write::GzEncoder<W>);

impl<W: Write> Writer<W> {
    pub fn new(writer: W) -> std::io::Result<Self> {
        let gz = flate2::GzBuilder::new()
            .comment(GZIP_HEADER_COMMENT)
            .write(writer, flate2::Compression::best());
        Ok(Self(gz))
    }

    pub fn write(&mut self, mut bytes: &[u8], timestamp: SystemTime) -> std::io::Result<()> {
        while bytes.len() > u16::MAX as usize {
            let (left, right) = bytes.split_at(u16::MAX as usize);
            self.write(left, timestamp)?;
            bytes = right;
        }

        assert!(bytes.len() <= u16::MAX as usize);

        let mut buffer = vec![0u8; bytes.len() + size_of::<Record>()];
        let (record, data) = buffer.as_mut_slice().split_at_mut(size_of::<Record>());
        let record = Record::mut_from_bytes(record).unwrap();
        record.set_timestamp(timestamp);
        record.data_length.set(bytes.len() as u16);
        data.copy_from_slice(bytes);

        self.0.write_all(&buffer)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }

    pub fn finish(self) -> std::io::Result<W> {
        self.0.finish()
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable,
)]
#[repr(C)]
struct Record {
    /// Number of data bytes in this block
    pub data_length: big_endian::U16,
    // Milliseconds since epoch
    pub timestamp: big_endian::U64,
}

impl Record {
    pub fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH.add(Duration::from_millis(self.timestamp.get()))
    }

    pub fn set_timestamp(&mut self, timestamp: SystemTime) {
        self.timestamp
            .set(timestamp.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
    }
}
