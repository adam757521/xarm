use crate::types::base::Status;
use strum::{AsRefStr, FromRepr};

// TODO: improve this whole system

#[repr(usize)]
#[derive(Debug, AsRefStr, FromRepr, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    EfiLoadError = 1,

    InvalidParameter = 2,

    Unsupported = 3,

    BadBufferSize = 4,
    BufferTooSmall = 5,

    NotReady = 6,

    DeviceError = 7,

    WriteProtected = 8,

    OutOfResources = 9,

    VolumeCorrupted = 10,
    VolumeFull = 11,

    NoMedia = 12,
    MediaChanged = 13,

    NotFound = 14,
    AccessDenied = 15,

    NoResponse = 16,
    NoMapping = 17,

    Timeout = 18,
    NotStarted = 19,
    AlreadyStarted = 20,
    Aborted = 21,

    IcmpError = 22,
    TftpError = 23,
    ProtocolError = 24,

    IncompatibleVersion = 25,
    SecurityViolation = 26,
    CrcError = 27,

    EndOfMedia = 28,

    EndOfFile = 31,

    InvalidLanguage = 32,

    CompromisedData = 33,

    IpAddressConflict = 34,
    HttpError = 35,

    UnknownError,
}

pub struct StatusError {
    pub code: Status,
    pub error: Error,
}

pub type Result<T> = core::result::Result<T, StatusError>;

impl Into<StatusError> for Error {
    fn into(self) -> StatusError {
        StatusError {
            code: Status(self as usize),
            error: self,
        }
    }
}

impl From<Status> for Option<Error> {
    fn from(code: Status) -> Self {
        let s = ((code.0 as usize) << 4) >> 4;

        match s {
            0 => None,
            1 => Some(Error::EfiLoadError),
            2 => Some(Error::InvalidParameter),
            3 => Some(Error::Unsupported),
            4 => Some(Error::BadBufferSize),
            5 => Some(Error::BufferTooSmall),
            6 => Some(Error::NotReady),
            7 => Some(Error::DeviceError),
            8 => Some(Error::WriteProtected),
            9 => Some(Error::OutOfResources),
            10 => Some(Error::VolumeCorrupted),
            11 => Some(Error::VolumeFull),
            12 => Some(Error::NoMedia),
            13 => Some(Error::MediaChanged),
            14 => Some(Error::NotFound),
            15 => Some(Error::AccessDenied),
            16 => Some(Error::NoResponse),
            17 => Some(Error::NoMapping),
            18 => Some(Error::Timeout),
            19 => Some(Error::NotStarted),
            20 => Some(Error::AlreadyStarted),
            21 => Some(Error::Aborted),
            22 => Some(Error::IcmpError),
            23 => Some(Error::TftpError),
            24 => Some(Error::ProtocolError),
            25 => Some(Error::IncompatibleVersion),
            26 => Some(Error::SecurityViolation),
            27 => Some(Error::CrcError),
            28 => Some(Error::EndOfMedia),
            31 => Some(Error::EndOfFile),
            32 => Some(Error::InvalidLanguage),
            33 => Some(Error::CompromisedData),
            34 => Some(Error::IpAddressConflict),
            35 => Some(Error::HttpError),
            _ => Some(Error::UnknownError),
        }

        /*
        if code.0 == 0 {
            return Ok(());
        }

        Err(Error::from_repr(code.0).unwrap_or(Error::UnknownError))*/
    }
}

impl From<Status> for Result<()> {
    fn from(c: Status) -> Self {
        match Option::<Error>::from(c) {
            None => Ok(()),
            Some(err) => Err(StatusError {
                code: c,
                error: err,
            }),
        }
    }
}
