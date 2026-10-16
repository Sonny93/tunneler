use std::io;

use serde::Serialize;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt};

/// The longest line, newline excluded, either side accepts: 1 MiB.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

const LINE_TERMINATOR: u8 = b'\n';

/// What reading one line produced.
#[derive(Debug, PartialEq, Eq)]
pub enum LineRead {
    /// A complete line, without its terminator.
    Line(Vec<u8>),
    /// The peer closed the stream, possibly in the middle of a line.
    EndOfStream,
    /// The line exceeds the limit; the rest of it was not read.
    TooLong,
}

/// Reads up to the next newline, giving up past `max_line_bytes`.
///
/// # Errors
///
/// Fails when the underlying stream fails.
pub async fn read_line<R>(reader: &mut R, max_line_bytes: usize) -> io::Result<LineRead>
where
    R: AsyncBufRead + AsyncRead + Unpin,
{
    let limit = u64::try_from(max_line_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut line = Vec::new();
    let read_bytes = (&mut *reader)
        .take(limit)
        .read_until(LINE_TERMINATOR, &mut line)
        .await?;
    if line.last() == Some(&LINE_TERMINATOR) {
        line.pop();
        return Ok(LineRead::Line(line));
    }
    if read_bytes > max_line_bytes {
        return Ok(LineRead::TooLong);
    }
    Ok(LineRead::EndOfStream)
}

/// Serializes a message as one line, terminator included.
///
/// # Errors
///
/// Fails when the message cannot be represented as JSON.
pub fn encode_line<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(message)?;
    line.push(char::from(LINE_TERMINATOR));
    Ok(line)
}

#[cfg(test)]
mod tests {
    use tokio::io::BufReader;

    use super::*;

    async fn read_from(input: &[u8], max_line_bytes: usize) -> LineRead {
        let mut reader = BufReader::new(input);
        read_line(&mut reader, max_line_bytes).await.unwrap()
    }

    #[tokio::test]
    async fn should_read_a_line_without_its_terminator() {
        assert_eq!(
            read_from(b"abc\n", 10).await,
            LineRead::Line(b"abc".to_vec())
        );
    }

    #[tokio::test]
    async fn should_read_a_line_of_exactly_the_limit() {
        assert_eq!(
            read_from(b"abcd\n", 4).await,
            LineRead::Line(b"abcd".to_vec())
        );
    }

    #[tokio::test]
    async fn should_report_too_long_when_the_line_exceeds_the_limit() {
        assert_eq!(read_from(b"abcde\n", 4).await, LineRead::TooLong);
    }

    #[tokio::test]
    async fn should_report_too_long_when_the_terminator_never_comes() {
        assert_eq!(read_from(b"abcdefgh", 4).await, LineRead::TooLong);
    }

    #[tokio::test]
    async fn should_report_end_of_stream_when_the_stream_is_empty() {
        assert_eq!(read_from(b"", 4).await, LineRead::EndOfStream);
    }

    #[tokio::test]
    async fn should_report_end_of_stream_when_the_last_line_is_unterminated() {
        assert_eq!(read_from(b"abc", 10).await, LineRead::EndOfStream);
    }

    #[tokio::test]
    async fn should_read_consecutive_lines_one_at_a_time() {
        let mut reader = BufReader::new(&b"one\ntwo\n"[..]);

        let first = read_line(&mut reader, 10).await.unwrap();
        let second = read_line(&mut reader, 10).await.unwrap();

        assert_eq!(
            (first, second),
            (
                LineRead::Line(b"one".to_vec()),
                LineRead::Line(b"two".to_vec())
            )
        );
    }

    #[test]
    fn should_end_an_encoded_line_with_a_newline() {
        assert_eq!(encode_line(&"hi").unwrap(), "\"hi\"\n");
    }
}
