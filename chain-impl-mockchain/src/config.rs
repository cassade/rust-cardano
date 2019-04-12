use crate::block::ConsensusVersion;
use crate::leadership::bft::LeaderId;
use crate::milli::Milli;
use chain_addr::Discrimination;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::packer::Codec;
use chain_core::property;
use chain_crypto::{bech32::Bech32 as _, PublicKey};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// Possible errors
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    InvalidTag,
    SizeInvalid,
    StructureInvalid,
    UnknownString(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::InvalidTag => write!(f, "Invalid config parameter tag"),
            Error::SizeInvalid => write!(f, "Invalid config parameter size"),
            Error::StructureInvalid => write!(f, "Invalid config parameter structure"),
            Error::UnknownString(s) => write!(f, "Invalid config parameter string '{}'", s),
        }
    }
}

impl std::error::Error for Error {}

impl Into<ReadError> for Error {
    fn into(self) -> ReadError {
        ReadError::StructureInvalid(self.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigParam {
    Block0Date(Block0Date),
    Discrimination(Discrimination),
    ConsensusVersion(ConsensusVersion),
    SlotsPerEpoch(u64),
    SlotDuration(u8),
    ConsensusLeaderCert(LeaderId),
    ConsensusGenesisPraosParamD(Milli),
    ConsensusGenesisPraosParamF(Milli),
}

// Discriminants can NEVER be 1024 or higher
#[derive(AsRefStr, Clone, Copy, Debug, EnumIter, EnumString, FromPrimitive, PartialEq)]
enum Tag {
    #[strum(to_string = "block0-date")]
    Block0Date = 1,
    #[strum(to_string = "discrimination")]
    Discrimination = 2,
    #[strum(to_string = "block0-consensus")]
    ConsensusVersion = 3,
    #[strum(to_string = "slots-per-epoch")]
    SlotsPerEpoch = 4,
    #[strum(to_string = "slot-duration")]
    SlotDuration = 5,
    #[strum(to_string = "block0-consensus-leader")]
    ConsensusLeaderCert = 6,
    #[strum(to_string = "genesis-praos-param-d")]
    ConsensusGenesisPraosParamD = 7,
    #[strum(to_string = "genesis-praos-param-f")]
    ConsensusGenesisPraosParamF = 8,
}

impl<'a> From<&'a ConfigParam> for Tag {
    fn from(config_param: &'a ConfigParam) -> Self {
        match config_param {
            ConfigParam::Block0Date(_) => Tag::Block0Date,
            ConfigParam::Discrimination(_) => Tag::Discrimination,
            ConfigParam::ConsensusVersion(_) => Tag::ConsensusVersion,
            ConfigParam::SlotsPerEpoch(_) => Tag::SlotsPerEpoch,
            ConfigParam::SlotDuration(_) => Tag::SlotDuration,
            ConfigParam::ConsensusLeaderCert(_) => Tag::ConsensusLeaderCert,
            ConfigParam::ConsensusGenesisPraosParamD(_) => Tag::ConsensusGenesisPraosParamD,
            ConfigParam::ConsensusGenesisPraosParamF(_) => Tag::ConsensusGenesisPraosParamF,
        }
    }
}

impl Readable for ConfigParam {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let taglen = TagLen(buf.get_u16()?);
        let bytes = buf.get_slice(taglen.get_len())?;
        match taglen.get_tag().map_err(Into::into)? {
            Tag::Block0Date => ConfigParamVariant::from_payload(bytes).map(ConfigParam::Block0Date),
            Tag::Discrimination => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::Discrimination)
            }
            Tag::ConsensusVersion => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::ConsensusVersion)
            }
            Tag::SlotsPerEpoch => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::SlotsPerEpoch)
            }
            Tag::SlotDuration => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::SlotDuration)
            }
            Tag::ConsensusLeaderCert => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::ConsensusLeaderCert)
            }
            Tag::ConsensusGenesisPraosParamD => ConfigParamVariant::from_payload(bytes)
                .map(ConfigParam::ConsensusGenesisPraosParamD),
            Tag::ConsensusGenesisPraosParamF => ConfigParamVariant::from_payload(bytes)
                .map(ConfigParam::ConsensusGenesisPraosParamF),
        }
        .map_err(Into::into)
    }
}

impl property::Serialize for ConfigParam {
    type Error = io::Error;

    fn serialize<W: Write>(&self, writer: W) -> Result<(), Self::Error> {
        let tag = Tag::from(self);
        let bytes = match self {
            ConfigParam::Block0Date(data) => data.to_payload(),
            ConfigParam::Discrimination(data) => data.to_payload(),
            ConfigParam::ConsensusVersion(data) => data.to_payload(),
            ConfigParam::SlotsPerEpoch(data) => data.to_payload(),
            ConfigParam::SlotDuration(data) => data.to_payload(),
            ConfigParam::ConsensusLeaderCert(data) => data.to_payload(),
            ConfigParam::ConsensusGenesisPraosParamD(data) => data.to_payload(),
            ConfigParam::ConsensusGenesisPraosParamF(data) => data.to_payload(),
        };
        let taglen = TagLen::new(tag, bytes.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "initial ent payload too big".to_string(),
            )
        })?;
        let mut codec = Codec::from(writer);
        codec.put_u16(taglen.0)?;
        codec.write_all(&bytes)
    }
}

#[cfg(feature = "generic-serialization")]
mod serde_impl {
    use super::*;
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    impl<'de> Deserialize<'de> for ConfigParam {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let (tag_str, value) = <(String, String)>::deserialize(deserializer)?;
            let tag = Tag::from_str(tag_str).map_err(|_| D::Error::custom(Error::InvalidTag))?;
            match tag {
                Tag::Block0Date => Block0Date::from_cfg_str(&value).map(ConfigParam::Block0Date),
                Tag::Discrimination => {
                    Discrimination::from_cfg_str(&value).map(ConfigParam::Discrimination)
                }
                Tag::ConsensusVersion => {
                    ConsensusVersion::from_cfg_str(&value).map(ConfigParam::ConsensusVersion)
                }
                Tag::SlotsPerEpoch => {
                    SlotsPerEpoch::from_cfg_str(&value).map(ConfigParam::SlotsPerEpoch)
                }
                Tag::SlotDuration => {
                    SlotDuration::from_cfg_str(&value).map(ConfigParam::SlotDuration)
                }
                Tag::ConsensusLeaderCert => {
                    ConsensusLeaderCert::from_cfg_str(&value).map(ConfigParam::ConsensusLeaderCert)
                }
                Tag::ConsensusGenesisPraosParamD => {
                    ConsensusGenesisPraosParamD::from_cfg_str(&value)
                        .map(ConfigParam::ConsensusGenesisPraosParamD)
                }
                Tag::ConsensusGenesisPraosParamF => {
                    ConsensusGenesisPraosParamF::from_cfg_str(&value)
                        .map(ConfigParam::ConsensusGenesisPraosParamF)
                }
            }
            .map_err(D::Error::custom)
        }
    }

    impl Serialize for ConfigParam {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let tag = Tag::from(&self).as_ref();
            let value = match self {
                ConfigParam::Block0Date(data) => data.to_cfg_string(),
                ConfigParam::Discrimination(data) => data.to_cfg_string(),
                ConfigParam::ConsensusVersion(data) => data.to_cfg_string(),
                ConfigParam::SlotsPerEpoch(data) => data.to_cfg_string(),
                ConfigParam::SlotDuration(data) => data.to_cfg_string(),
                ConfigParam::ConsensusLeaderCert(data) => data.to_cfg_string(),
                ConfigParam::ConsensusGenesisPraosParamD(data) => data.to_cfg_string(),
                ConfigParam::ConsensusGenesisPraosParamF(data) => data.to_cfg_string(),
            };
            (tag, value).serialize(serializer)
        }
    }
}

trait ConfigParamVariant: Clone + Eq + PartialEq {
    fn to_payload(&self) -> Vec<u8>;
    fn from_payload(payload: &[u8]) -> Result<Self, Error>;
    fn to_cfg_string(&self) -> String;
    fn from_cfg_str(s: &str) -> Result<Self, Error>;
}

/// Seconds elapsed since 1-Jan-1970 (unix time)
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Block0Date(pub u64);

impl ConfigParamVariant for Block0Date {
    fn to_payload(&self) -> Vec<u8> {
        self.0.to_payload()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        u64::from_payload(payload).map(Block0Date)
    }

    fn to_cfg_string(&self) -> String {
        self.0.to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        from_cfg_str(s).map(Block0Date)
    }
}

const VAL_PROD: u8 = 1;
const VAL_TEST: u8 = 2;

impl ConfigParamVariant for Discrimination {
    fn to_payload(&self) -> Vec<u8> {
        match self {
            Discrimination::Production => vec![VAL_PROD],
            Discrimination::Test => vec![VAL_TEST],
        }
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() != 1 {
            return Err(Error::SizeInvalid);
        };
        match payload[0] {
            VAL_PROD => Ok(Discrimination::Production),
            VAL_TEST => Ok(Discrimination::Test),
            _ => Err(Error::StructureInvalid),
        }
    }

    fn to_cfg_string(&self) -> String {
        match self {
            Discrimination::Production => "production",
            Discrimination::Test => "test",
        }
        .to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        match s {
            "production" => Ok(Discrimination::Production),
            "test" => Ok(Discrimination::Test),
            _ => Err(Error::UnknownString(s.to_string())),
        }
    }
}

impl ConfigParamVariant for ConsensusVersion {
    fn to_payload(&self) -> Vec<u8> {
        (*self as u16).to_be_bytes().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut bytes = 0u16.to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(payload);
        let integer = u16::from_be_bytes(bytes);
        ConsensusVersion::from_u16(integer).ok_or(Error::StructureInvalid)
    }

    fn to_cfg_string(&self) -> String {
        self.to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        from_cfg_str(s)
    }
}

impl ConfigParamVariant for LeaderId {
    fn to_payload(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        PublicKey::from_binary(payload)
            .map(Into::into)
            .map_err(|_| Error::SizeInvalid)
    }

    fn to_cfg_string(&self) -> String {
        self.as_public_key().to_bech32_str()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        PublicKey::try_from_bech32_str(s)
            .map(Into::into)
            .map_err(|_| Error::UnknownString(s.to_string()))
    }
}

impl ConfigParamVariant for u8 {
    fn to_payload(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        match payload.len() {
            1 => Ok(payload[0]),
            _ => Err(Error::SizeInvalid),
        }
    }

    fn to_cfg_string(&self) -> String {
        self.to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        from_cfg_str(s)
    }
}

impl ConfigParamVariant for u64 {
    fn to_payload(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut bytes = Self::default().to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(payload);
        Ok(Self::from_be_bytes(bytes))
    }

    fn to_cfg_string(&self) -> String {
        self.to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        from_cfg_str(s)
    }
}

impl ConfigParamVariant for Milli {
    fn to_payload(&self) -> Vec<u8> {
        self.into_millis().to_payload()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        u64::from_payload(payload).map(Milli::from_millis)
    }

    fn to_cfg_string(&self) -> String {
        self.to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        from_cfg_str(s)
    }
}

fn from_cfg_str<T: FromStr>(s: &str) -> Result<T, Error> {
    s.parse().map_err(|_| Error::UnknownString(s.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TagLen(u16);

const MAXIMUM_LEN: usize = 64;

impl TagLen {
    pub fn new(tag: Tag, len: usize) -> Option<Self> {
        if len < MAXIMUM_LEN {
            Some(TagLen((tag as u16) << 6 | len as u16))
        } else {
            None
        }
    }

    pub fn get_len(self) -> usize {
        (self.0 & 0b11_1111) as usize
    }

    pub fn get_tag(self) -> Result<Tag, Error> {
        FromPrimitive::from_u16(self.0 >> 6).ok_or(Error::InvalidTag)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use strum::IntoEnumIterator;

    quickcheck! {
        fn tag_len_computation_correct(tag: Tag, len: usize) -> TestResult {
            let len = len % MAXIMUM_LEN;
            let tag_len = TagLen::new(tag, len).unwrap();

            assert_eq!(Ok(tag), tag_len.get_tag(), "Invalid tag");
            assert_eq!(len, tag_len.get_len(), "Invalid len");
            TestResult::passed()
        }
    }

    impl Arbitrary for Tag {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let idx = usize::arbitrary(g) % Tag::iter().count();
            Tag::iter().nth(idx).unwrap()
        }
    }

    impl Arbitrary for Block0Date {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block0Date(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 8 {
                0 => ConfigParam::Block0Date(Arbitrary::arbitrary(g)),
                1 => ConfigParam::Discrimination(Arbitrary::arbitrary(g)),
                2 => ConfigParam::ConsensusVersion(Arbitrary::arbitrary(g)),
                3 => ConfigParam::SlotsPerEpoch(Arbitrary::arbitrary(g)),
                4 => ConfigParam::SlotDuration(Arbitrary::arbitrary(g)),
                5 => ConfigParam::ConsensusLeaderCert(Arbitrary::arbitrary(g)),
                6 => ConfigParam::ConsensusGenesisPraosParamD(Arbitrary::arbitrary(g)),
                7 => ConfigParam::ConsensusGenesisPraosParamF(Arbitrary::arbitrary(g)),
                _ => unreachable!(),
            }
        }
    }
}
