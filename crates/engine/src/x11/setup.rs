use std::io;

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::cookie::{MIT_MAGIC_COOKIE_NAME, constant_time_equals};

const SETUP_HEADER_LENGTH: usize = 12;
const BIG_ENDIAN_MARKER: u8 = 0x42;
const LITTLE_ENDIAN_MARKER: u8 = 0x6c;
const NAME_LENGTH_OFFSET: usize = 6;
const DATA_LENGTH_OFFSET: usize = 8;
const FIELD_PADDING_UNIT: usize = 4;
const MAX_AUTH_FIELD_LENGTH: usize = 256;

/// Why the setup packet of an X client cannot be forwarded.
#[derive(Debug, Error)]
pub(crate) enum SetupError {
    #[error("cannot read the X11 setup packet: {0}")]
    Read(io::Error),
    #[error("the X11 setup packet is cut short")]
    Truncated,
    #[error("the X11 setup packet starts with the unknown byte order 0x{0:02x}")]
    UnknownByteOrder(u8),
    #[error("the X11 setup packet carries an authorization field of {0} bytes")]
    FieldTooLong(usize),
    #[error("the X client did not use the MIT-MAGIC-COOKIE-1 protocol")]
    UnexpectedProtocol,
    #[error("the X client presented a cookie other than the one given to the server")]
    CookieMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    Big,
    Little,
}

impl ByteOrder {
    fn from_marker(marker: u8) -> Result<Self, SetupError> {
        match marker {
            BIG_ENDIAN_MARKER => Ok(Self::Big),
            LITTLE_ENDIAN_MARKER => Ok(Self::Little),
            unknown => Err(SetupError::UnknownByteOrder(unknown)),
        }
    }

    fn read_u16(self, bytes: [u8; 2]) -> u16 {
        match self {
            Self::Big => u16::from_be_bytes(bytes),
            Self::Little => u16::from_le_bytes(bytes),
        }
    }

    fn write_u16(self, value: u16) -> [u8; 2] {
        match self {
            Self::Big => value.to_be_bytes(),
            Self::Little => value.to_le_bytes(),
        }
    }
}

type HeaderBytes = [u8; SETUP_HEADER_LENGTH];

struct SetupHeader {
    byte_order: ByteOrder,
    name_length: usize,
    data_length: usize,
}

impl SetupHeader {
    fn parse(header: &HeaderBytes) -> Result<Self, SetupError> {
        let byte_order = ByteOrder::from_marker(header[0])?;
        let name_length = usize::from(
            byte_order.read_u16([header[NAME_LENGTH_OFFSET], header[NAME_LENGTH_OFFSET + 1]]),
        );
        let data_length = usize::from(
            byte_order.read_u16([header[DATA_LENGTH_OFFSET], header[DATA_LENGTH_OFFSET + 1]]),
        );
        for length in [name_length, data_length] {
            if length > MAX_AUTH_FIELD_LENGTH {
                return Err(SetupError::FieldTooLong(length));
            }
        }
        Ok(Self {
            byte_order,
            name_length,
            data_length,
        })
    }

    fn data_start(&self) -> usize {
        SETUP_HEADER_LENGTH + padded(self.name_length)
    }

    fn prefix_length(&self) -> usize {
        self.data_start() + padded(self.data_length)
    }
}

fn padded(length: usize) -> usize {
    length.div_ceil(FIELD_PADDING_UNIT) * FIELD_PADDING_UNIT
}

/// Reads exactly the setup prefix an X client sends first: the fixed header
/// and the padded authorization name and data. Nothing after it is consumed.
pub(crate) async fn read_setup_prefix<Reader>(reader: &mut Reader) -> Result<Vec<u8>, SetupError>
where
    Reader: AsyncRead + Unpin,
{
    let mut header = [0_u8; SETUP_HEADER_LENGTH];
    reader
        .read_exact(&mut header)
        .await
        .map_err(SetupError::Read)?;
    let prefix_length = SetupHeader::parse(&header)?.prefix_length();
    let mut prefix = header.to_vec();
    prefix.resize(prefix_length, 0);
    reader
        .read_exact(&mut prefix[SETUP_HEADER_LENGTH..])
        .await
        .map_err(SetupError::Read)?;
    Ok(prefix)
}

/// Checks that the prefix presents the cookie given to the server, then
/// builds the prefix to send to the local X server: the same header with the
/// real authorization, or none when there is no real cookie.
pub(crate) fn rewrite_setup_prefix(
    prefix: &[u8],
    fake_cookie: &[u8],
    real_cookie: Option<&[u8]>,
) -> Result<Vec<u8>, SetupError> {
    let header: HeaderBytes = prefix
        .get(..SETUP_HEADER_LENGTH)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(SetupError::Truncated)?;
    let parsed = SetupHeader::parse(&header)?;
    let name = prefix
        .get(SETUP_HEADER_LENGTH..SETUP_HEADER_LENGTH + parsed.name_length)
        .ok_or(SetupError::Truncated)?;
    let data = prefix
        .get(parsed.data_start()..parsed.data_start() + parsed.data_length)
        .ok_or(SetupError::Truncated)?;
    if name != MIT_MAGIC_COOKIE_NAME {
        return Err(SetupError::UnexpectedProtocol);
    }
    if !constant_time_equals(data, fake_cookie) {
        return Err(SetupError::CookieMismatch);
    }
    build_prefix(header, parsed.byte_order, real_cookie)
}

fn build_prefix(
    mut header: HeaderBytes,
    byte_order: ByteOrder,
    real_cookie: Option<&[u8]>,
) -> Result<Vec<u8>, SetupError> {
    let (name, data) = match real_cookie {
        Some(cookie) => (MIT_MAGIC_COOKIE_NAME, cookie),
        None => (&[][..], &[][..]),
    };
    write_length(&mut header, NAME_LENGTH_OFFSET, byte_order, name.len())?;
    write_length(&mut header, DATA_LENGTH_OFFSET, byte_order, data.len())?;
    let mut prefix = header.to_vec();
    append_padded(&mut prefix, name);
    append_padded(&mut prefix, data);
    Ok(prefix)
}

fn write_length(
    header: &mut HeaderBytes,
    offset: usize,
    byte_order: ByteOrder,
    length: usize,
) -> Result<(), SetupError> {
    let length = u16::try_from(length).map_err(|_| SetupError::FieldTooLong(length))?;
    header[offset..offset + 2].copy_from_slice(&byte_order.write_u16(length));
    Ok(())
}

fn append_padded(prefix: &mut Vec<u8>, field: &[u8]) {
    prefix.extend_from_slice(field);
    prefix.resize(prefix.len() + padded(field.len()) - field.len(), 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE: [u8; 16] = [0x11; 16];
    const REAL: [u8; 16] = [0x22; 16];
    const NAME: &[u8] = b"MIT-MAGIC-COOKIE-1";
    const NAME_PADDING: usize = 2;

    fn client_prefix(byte_order: ByteOrder, name: &[u8], data: &[u8]) -> Vec<u8> {
        let marker = match byte_order {
            ByteOrder::Big => BIG_ENDIAN_MARKER,
            ByteOrder::Little => LITTLE_ENDIAN_MARKER,
        };
        let mut prefix = vec![marker, 0];
        prefix.extend(byte_order.write_u16(11));
        prefix.extend(byte_order.write_u16(0));
        prefix.extend(byte_order.write_u16(u16::try_from(name.len()).unwrap()));
        prefix.extend(byte_order.write_u16(u16::try_from(data.len()).unwrap()));
        prefix.extend([0, 0]);
        append_padded(&mut prefix, name);
        append_padded(&mut prefix, data);
        prefix
    }

    fn rewrite(prefix: &[u8], real: Option<&[u8]>) -> Result<Vec<u8>, SetupError> {
        rewrite_setup_prefix(prefix, &FAKE, real)
    }

    #[test]
    fn should_swap_the_cookie_when_the_client_is_big_endian() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        let rewritten = rewrite(&prefix, Some(&REAL)).unwrap();

        assert_eq!(rewritten, client_prefix(ByteOrder::Big, NAME, &REAL));
    }

    #[test]
    fn should_swap_the_cookie_when_the_client_is_little_endian() {
        let prefix = client_prefix(ByteOrder::Little, NAME, &FAKE);

        let rewritten = rewrite(&prefix, Some(&REAL)).unwrap();

        assert_eq!(rewritten, client_prefix(ByteOrder::Little, NAME, &REAL));
    }

    #[test]
    fn should_pad_the_name_to_four_bytes_when_rewriting() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        let rewritten = rewrite(&prefix, Some(&REAL)).unwrap();

        assert_eq!(
            rewritten.len(),
            SETUP_HEADER_LENGTH + NAME.len() + NAME_PADDING + REAL.len()
        );
    }

    #[test]
    fn should_swap_for_a_cookie_of_another_length_when_the_real_one_is_longer() {
        let longer = [0x33_u8; 21];
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        let rewritten = rewrite(&prefix, Some(&longer)).unwrap();

        assert_eq!(rewritten, client_prefix(ByteOrder::Big, NAME, &longer));
    }

    #[test]
    fn should_send_an_empty_authorization_when_there_is_no_real_cookie() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        let rewritten = rewrite(&prefix, None).unwrap();

        assert_eq!(rewritten, client_prefix(ByteOrder::Big, &[], &[]));
    }

    #[test]
    fn should_send_an_empty_authorization_when_there_is_no_real_cookie_and_the_client_is_little_endian()
     {
        let prefix = client_prefix(ByteOrder::Little, NAME, &FAKE);

        let rewritten = rewrite(&prefix, None).unwrap();

        assert_eq!(rewritten, client_prefix(ByteOrder::Little, &[], &[]));
    }

    #[test]
    fn should_keep_the_protocol_version_when_rewriting() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        let rewritten = rewrite(&prefix, Some(&REAL)).unwrap();

        assert_eq!(rewritten[..6], prefix[..6]);
    }

    #[test]
    fn should_reject_the_client_when_the_cookie_is_wrong() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &[0x99; 16]);

        assert!(matches!(
            rewrite(&prefix, Some(&REAL)),
            Err(SetupError::CookieMismatch)
        ));
    }

    #[test]
    fn should_reject_the_client_when_the_cookie_is_empty() {
        let prefix = client_prefix(ByteOrder::Big, &[], &[]);

        assert!(matches!(
            rewrite(&prefix, Some(&REAL)),
            Err(SetupError::UnexpectedProtocol)
        ));
    }

    #[test]
    fn should_reject_the_client_when_the_protocol_name_is_wrong() {
        let prefix = client_prefix(ByteOrder::Big, b"XDM-AUTHORIZATION-1", &FAKE);

        assert!(matches!(
            rewrite(&prefix, Some(&REAL)),
            Err(SetupError::UnexpectedProtocol)
        ));
    }

    #[test]
    fn should_reject_the_client_when_the_byte_order_is_unknown() {
        let mut prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);
        prefix[0] = b'X';

        assert!(matches!(
            rewrite(&prefix, Some(&REAL)),
            Err(SetupError::UnknownByteOrder(b'X'))
        ));
    }

    #[test]
    fn should_reject_the_client_when_an_authorization_field_is_oversized() {
        let oversized = vec![0_u8; MAX_AUTH_FIELD_LENGTH + 1];
        let prefix = client_prefix(ByteOrder::Big, NAME, &oversized);

        assert!(matches!(
            rewrite(&prefix, Some(&REAL)),
            Err(SetupError::FieldTooLong(_))
        ));
    }

    #[test]
    fn should_reject_the_prefix_when_it_is_shorter_than_its_header_announces() {
        let prefix = client_prefix(ByteOrder::Big, NAME, &FAKE);

        assert!(matches!(
            rewrite(&prefix[..prefix.len() - 8], Some(&REAL)),
            Err(SetupError::Truncated)
        ));
    }

    #[test]
    fn should_reject_the_prefix_when_it_is_shorter_than_the_header() {
        assert!(matches!(
            rewrite(&[BIG_ENDIAN_MARKER, 0, 0], Some(&REAL)),
            Err(SetupError::Truncated)
        ));
    }

    #[tokio::test]
    async fn should_read_only_the_prefix_when_more_bytes_follow() {
        let mut bytes = client_prefix(ByteOrder::Big, NAME, &FAKE);
        let prefix_length = bytes.len();
        bytes.extend(b"requests");
        let mut reader = &bytes[..];

        let prefix = read_setup_prefix(&mut reader).await.unwrap();

        assert_eq!((prefix.len(), reader), (prefix_length, &b"requests"[..]));
    }

    #[tokio::test]
    async fn should_fail_to_read_when_the_stream_ends_inside_the_prefix() {
        let bytes = client_prefix(ByteOrder::Big, NAME, &FAKE);
        let mut reader = &bytes[..20];

        assert!(matches!(
            read_setup_prefix(&mut reader).await,
            Err(SetupError::Read(_))
        ));
    }

    #[tokio::test]
    async fn should_not_read_the_body_when_the_header_is_oversized() {
        let oversized = vec![0_u8; MAX_AUTH_FIELD_LENGTH + 1];
        let bytes = client_prefix(ByteOrder::Big, NAME, &oversized);
        let mut reader = &bytes[..SETUP_HEADER_LENGTH];

        assert!(matches!(
            read_setup_prefix(&mut reader).await,
            Err(SetupError::FieldTooLong(_))
        ));
    }
}
