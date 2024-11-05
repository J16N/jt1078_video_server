use crate::helper::{get_bit_at, get_num_at};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

type Result<T> = std::result::Result<T, std::io::Error>;

#[derive(Serialize, Deserialize)]
pub struct RtpPacket {
    pub(crate) header: RtpHeader,
    pub(crate) payload: Vec<u8>,
}

impl RtpPacket {
    pub async fn parse<T>(reader: &mut T) -> Result<Self>
    where
        T: AsyncBufReadExt + std::marker::Unpin,
    {
        let header = RtpHeader::parse(reader).await?;
        let mut payload = vec![0x00; header.data_body_length];
        reader.read_exact(&mut payload).await?;
        Ok(Self { header, payload })
    }
}

#[derive(Serialize, Deserialize)]
pub struct RtpHeader {
    pub(crate) version: String,
    pub(crate) padding: bool,
    pub(crate) extension_bit: bool,
    pub(crate) csrc_count: u8,
    pub(crate) marker: bool,
    pub(crate) payload_type: String,
    pub(crate) package_serial_number: u16,
    pub(crate) terminal_serial_number: String,
    pub(crate) logical_channel_number: u8,
    pub(crate) data_type: String,
    pub(crate) subpacket_processing_flag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_i_frame_interval: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_frame_interval: Option<u16>,
    pub(crate) data_body_length: usize,
}

impl RtpHeader {
    async fn parse<T>(reader: &mut T) -> Result<Self>
    where
        T: AsyncBufReadExt + std::marker::Unpin,
    {
        let mut buffer = [0; 16];
        reader.read_exact(&mut buffer).await?;

        // Parse header id. If it's not 0x30316364, return an error.
        Self::parse_header_id(&buffer[0..4])?;

        let num = buffer[4];
        let version = Self::parse_version(num)?;
        let padding = Self::parse_padding(num)?;
        let extension_bit = Self::parse_extension_bit(num)?;
        let csrc_count = Self::parse_csrc_count(num)?;

        let num = buffer[5];
        let marker = Self::parse_marker(num)?;
        let payload_type = Self::parse_payload_type(num)?;

        let package_serial_number = Self::parse_package_serial_number(&buffer[6..8])?;

        let terminal_serial_number_len = 6;
        let terminal_serial_number =
            Self::parse_terminal_serial_number(&buffer[8..(8 + terminal_serial_number_len)]);

        let logical_channel_number_idx = 8 + terminal_serial_number_len;
        let logical_channel_number = buffer[logical_channel_number_idx];

        let num = buffer[logical_channel_number_idx + 1];
        let mut data_type = 5;
        let type_of_data = Self::parse_data_type(num, &mut data_type)?;
        let subpacket_processing_flag = Self::parse_subpacket_flag(num)?;

        let buff_len = Self::get_remaining_data_length(data_type)?;
        let mut buffer = vec![0; buff_len];
        reader.read_exact(&mut buffer).await?;

        let mut timestamp = None;
        let mut last_i_frame_interval = None;
        let mut last_frame_interval = None;

        let mut timestamp_len = 0;
        let mut last_i_frame_interval_len = 0;
        let mut last_frame_interval_len = 0;

        if (0..4).contains(&data_type) {
            timestamp_len = 8;
            timestamp = Some(Self::parse_timestamp(&buffer[0..timestamp_len])?);
        }

        if (0..3).contains(&data_type) {
            last_i_frame_interval_len = 2;
            last_i_frame_interval = Some(Self::parse_last_i_frame_interval(
                &buffer[timestamp_len..(timestamp_len + last_i_frame_interval_len)],
            )?);
        }

        if (0..3).contains(&data_type) {
            last_frame_interval_len = 2;
            let offset = timestamp_len + last_i_frame_interval_len;
            last_frame_interval = Some(Self::parse_last_frame_interval(
                &buffer[offset..(offset + last_frame_interval_len)],
            )?)
        }

        let data_body_length_len = 2;
        let offset = timestamp_len + last_i_frame_interval_len + last_frame_interval_len;
        let data_body_length =
            Self::parse_data_body_length(&buffer[offset..(offset + data_body_length_len)])?;

        Ok(Self {
            version,
            padding,
            extension_bit,
            csrc_count,
            marker,
            payload_type,
            package_serial_number,
            terminal_serial_number,
            logical_channel_number,
            data_type: type_of_data,
            subpacket_processing_flag,
            timestamp,
            last_i_frame_interval,
            last_frame_interval,
            data_body_length,
        })
    }

    fn parse_header_id(bytees: &[u8]) -> Result<u32> {
        let bytees: [u8; 4] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'header_id' field",
                ))
            }
        };
        match u32::from_be_bytes(bytees) {
            0x30316364 => Ok(0x30316364),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid packet header",
            )),
        }
    }

    fn parse_version(bytee: u8) -> Result<String> {
        match get_num_at(bytee, 7, 2) {
            Ok(v) => Ok(v.to_string()),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'version' field",
            )),
        }
    }

    fn parse_padding(bytee: u8) -> Result<bool> {
        match get_bit_at(bytee, 5) {
            Ok(p) => Ok(p == 1),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'padding' field",
            )),
        }
    }

    fn parse_extension_bit(bytee: u8) -> Result<bool> {
        match get_bit_at(bytee, 4) {
            Ok(eb) => Ok(eb == 1),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'extension_bit' field",
            )),
        }
    }

    fn parse_csrc_count(bytee: u8) -> Result<u8> {
        match get_num_at(bytee, 3, 4) {
            Ok(cc) => Ok(cc),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'csrc_count' field",
            )),
        }
    }

    fn parse_marker(bytee: u8) -> Result<bool> {
        match get_bit_at(bytee, 7) {
            Ok(m) => Ok(m == 1),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'boundary' field",
            )),
        }
    }

    fn parse_payload_type(bytee: u8) -> Result<String> {
        let pt = match get_num_at(bytee, 6, 7) {
            Ok(98) => "H.264",
            Ok(99) => "H.265",
            Ok(6) => "G.711A",
            Ok(7) => "G.711U",
            Ok(_) | Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'payload_type' field",
                ))
            }
        };
        Ok(pt.to_string())
    }

    fn parse_package_serial_number(bytees: &[u8]) -> Result<u16> {
        let bytees: [u8; 2] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'package_serial_number' field",
                ))
            }
        };
        Ok(u16::from_be_bytes(bytees))
    }

    fn parse_terminal_serial_number(bytees: &[u8]) -> String {
        bytees.iter().fold(String::new(), |mut acc, &bytee| {
            acc.push_str(&format!("{:02X}", bytee));
            acc
        })
    }

    fn parse_data_type(bytee: u8, data_type: &mut u8) -> Result<String> {
        let types = [
            "Video I Frame",
            "Video P Frame",
            "Video B Frame",
            "Audio Frame",
            "Transparent Data Transmission",
        ];
        match get_num_at(bytee, 7, 4) {
            Ok(i) if (0..5).contains(&i) => {
                *data_type = i;
                Ok(types[i as usize].to_string())
            }
            Ok(_) | Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'type_of_data' field",
            )),
        }
    }

    fn parse_subpacket_flag(bytee: u8) -> Result<String> {
        let flags = [
            "Atomic Packet",
            "First Packet",
            "Last Packet",
            "Intermediate Packet",
        ];

        match get_num_at(bytee, 3, 4) {
            Ok(i) if (0..4).contains(&i) => Ok(flags[i as usize].to_string()),
            Ok(_) | Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse 'subpacket_processing_flag' field",
            )),
        }
    }

    fn get_remaining_data_length(bytee: u8) -> Result<usize> {
        match bytee {
            0..3 => Ok(14),
            3 => Ok(10),
            4 => Ok(2),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid 'data_type' field",
            )),
        }
    }

    fn parse_timestamp(bytees: &[u8]) -> Result<u64> {
        let bytees: [u8; 8] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'timestamp' field",
                ))
            }
        };
        Ok(u64::from_be_bytes(bytees))
    }

    fn parse_last_i_frame_interval(bytees: &[u8]) -> Result<u16> {
        let bytees: [u8; 2] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'last_i_frame_interval' field",
                ))
            }
        };
        Ok(u16::from_be_bytes(bytees))
    }

    fn parse_last_frame_interval(bytees: &[u8]) -> Result<u16> {
        let bytees: [u8; 2] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'last_frame_interval' field",
                ))
            }
        };
        Ok(u16::from_be_bytes(bytees))
    }

    fn parse_data_body_length(bytees: &[u8]) -> Result<usize> {
        let bytees: [u8; 2] = match bytees.try_into() {
            Ok(b) => b,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse 'data_body_length' field",
                ))
            }
        };
        Ok(u16::from_be_bytes(bytees) as usize)
    }
}
