use anyhow::{Result, anyhow};
use encoding_rs::{CoderResult, Encoding, UTF_8};
use ropey::{Rope, RopeBuilder, RopeSlice};
use std::io::Read;
use std::path::Path;

const BUF_SIZE: usize = 8192;

#[derive(Clone)]
pub struct Document {
    rope: Rope,
    encoding: &'static Encoding,
    has_bom: bool,
}

impl Document {
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn has_bom(&self) -> bool {
        self.has_bom
    }

    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line_num: usize) -> RopeSlice {
        self.rope.line(line_num)
    }

    pub fn line_idx(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    pub fn line_start(&self, char_idx: usize) -> usize {
        self.rope.line_to_char(char_idx)
    }

    pub fn char_to_byte_in_line(&self, line_offset: usize, line: RopeSlice) -> usize {
        line.char_to_byte(line_offset)
        // line.char_indices()
        //     .nth(line_offset)
        //     .map(|(b, _)| b)
        //     .unwrap_or(line.len())
    }

    // Read the first chunk of the file, look for the BOM (Byte Order Mark)
    // To figure out what the encoding is eg. UTF-8
    // Returns everthing need to decode the chunk
    fn read_and_detect_encoding<R: Read + ?Sized>(
        reader: &mut R,
        encoding: Option<&'static Encoding>,
        buf: &mut [u8],
    ) -> Result<(&'static Encoding, bool, encoding_rs::Decoder, usize)> {
        let bytes_read = reader.read(buf)?;
        let is_empty = bytes_read == 0;
        let (encoding, has_bom) = encoding
            .map(|e| (e, false)) // Try to override
            .or_else(|| Encoding::for_bom(&buf[..bytes_read]).map(|(e, _)| (e, true))) // Look for BOM
            .unwrap_or_else(|| {
                // If the two above fail we make a guess
                let mut det = chardetng::EncodingDetector::new();
                det.feed(&buf[..bytes_read], is_empty);
                (det.guess(None, true), false)
            });
        Ok((encoding, has_bom, encoding.new_decoder(), bytes_read))
    }

    pub fn from_reader<R: Read + ?Sized>(
        reader: &mut R,
        encoding: Option<&'static Encoding>,
    ) -> Result<(Rope, &'static Encoding, bool)> {
        let mut buf = [0u8; BUF_SIZE];
        let mut buf_out = [0u8; BUF_SIZE];
        let mut builder = RopeBuilder::new();

        let (encoding, has_bom, mut decoder, read) =
            Self::read_and_detect_encoding(reader, encoding, &mut buf)?;

        let mut slice = &buf[..read];
        let mut is_empty = read == 0;
        // SAFETY: zero-init array is valid UTF-8; decode only writes valid UTF-8.
        let buf_str = unsafe { std::str::from_utf8_unchecked_mut(&mut buf_out[..]) };
        let mut total_written = 0usize;

        loop {
            let mut total_read = 0usize;

            loop {
                let (result, r, w, _) = decoder.decode_to_str(
                    &slice[total_read..],
                    &mut buf_str[total_written..],
                    is_empty,
                );
                total_read += r;
                total_written += w;
                match result {
                    CoderResult::InputEmpty => break,
                    CoderResult::OutputFull => {
                        builder.append(&buf_str[..total_written]);
                        total_written = 0;
                    }
                }
            }
            if is_empty {
                builder.append(&buf_str[..total_written]);
                break;
            }
            let read = reader.read(&mut buf)?;
            slice = &buf[..read];
            is_empty = read == 0;
        }
        Ok((builder.finish(), encoding, has_bom))
    }

    pub fn open(path: &Path, encoding: Option<&'static Encoding>) -> Result<Self> {
        if path.metadata().is_ok_and(|m| !m.is_file()) {
            return Err(anyhow!("Target is not a file"));
        };
        if path.exists() {
            let mut file = std::fs::File::open(path)?;
            let (rope, reader_encoding, has_bom) = Self::from_reader(&mut file, encoding)?;
            Ok(Self {
                rope: rope,
                encoding: reader_encoding,
                has_bom: has_bom,
            })
        } else {
            Ok(Self {
                rope: Rope::from("\n"),
                encoding: encoding.unwrap_or(UTF_8),
                has_bom: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::{UTF_8, WINDOWS_1252};

    // Helper: run from_reader over in-memory bytes
    // &[u8] implements Read, so &mut &bytes[..] is a valid reader
    fn read(bytes: &[u8], enc: Option<&'static Encoding>) -> (Rope, &'static Encoding, bool) {
        Document::from_reader(&mut &bytes[..], enc).unwrap()
    }

    #[test]
    fn reads_plain_utf8() {
        let (rope, enc, bom) = read(b"hello\nworld\n", None);
        assert_eq!(rope.to_string(), "hello\nworld\n");
        assert_eq!(enc, UTF_8);
        assert!(!bom);
    }

    #[test]
    fn rope_returns_proper_line_count() {
        let (rope, _, _) = read(b"1line\n2line\n3line", None);
        assert_eq!(rope.len_lines(), 3);
    }

    #[test]
    fn rope_returns_proper_line() {
        let (rope, _, _) = read(b"Hello\nWorld\nCole\nIs\nGreat", None);
        assert_eq!(rope.line(3).to_string(), "Is\n".to_string());
    }

    #[test]
    fn empty_input_gives_rope() {
        let (rope, _, bom) = read(b"", None);
        assert_eq!(rope.to_string(), "");
        assert!(!bom);
    }

    #[test]
    fn multibyte_utf8_preserve() {
        let s = "café — 日本語 🚀\n";
        let (rope, _, _) = read(s.as_bytes(), None);
        assert_eq!(rope.to_string(), s);
    }

    #[test]
    fn utf8_bom_detected_and_stripped() {
        let mut data = vec![0xEF, 0xBB, 0xBF]; // This is just the BOM for UTF-8
        data.extend_from_slice("hello".as_bytes());
        let (rope, enc, bom) = read(&data, None);
        assert_eq!(enc, UTF_8);
        assert!(bom);
        assert_eq!(rope.to_string(), "hello");
    }

    #[test]
    fn input_larger_than_buffer_spans_chunks() {
        // BUF_SIZE (8192) exercises the OUTER loop / multiple disk reads
        let s = "abcsdfadsfadsfl\n".repeat(2000); // 22000 bytes
        let (rope, _, _) = read(s.as_bytes(), None);
        assert_eq!(rope.to_string(), s);
        assert_eq!(rope.len_lines(), 2001);
    }

    #[test]
    fn windows_1252_overrides_expands_bytes() {
        // 0xE9 = 'é' in Windows-1252: 1 input byte -> UTF-8 byts
        let (rope, enc, _) = read(&[b'a', 0xE9, b'z'], Some(WINDOWS_1252));
        assert_eq!(enc, WINDOWS_1252);
        assert_eq!(rope.to_string(), "aéz");
    }

    #[test]
    fn output_full_branch_when_decode_expands_past_buffer() {
        let (rope, _, _) = read(&vec![0xE9u8; 5000], Some(WINDOWS_1252));
        assert_eq!(rope.to_string(), "é".repeat(5000));
    }

    #[test]
    fn open_reads_existing_file() {
        let path = std::env::temp_dir().join("mist_open_existing.txt");
        std::fs::write(&path, "line1\nline2\nline3\n").unwrap();
        let document = Document::open(&path, None).unwrap();
        assert_eq!(document.rope().to_string(), "line1\nline2\nline3\n");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn open_missing_file_is_new_buffer() {
        let path = std::env::temp_dir().join("mist_open_missing.txt");
        std::fs::remove_file(&path).ok(); // make sure it's gone
        let document = Document::open(&path, None).unwrap();
        assert_eq!(document.rope().to_string(), "\n");
        assert!(!document.has_bom());
    }

    #[test]
    fn open_on_directory_errors() {
        assert!(Document::open(&std::env::temp_dir(), None).is_err());
    }
}
