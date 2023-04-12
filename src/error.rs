use std::fmt::{Debug, Display};

pub struct MrgError {
    pub msg: String,
}

impl Debug for MrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.msg);
    }
}

impl Display for MrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.msg);
    }
}

impl From<clap::error::Error> for MrgError {
    fn from(value: clap::error::Error) -> Self {
        return MrgError {
            msg: format!("cannot parse command line: {}", value),
        };
    }
}

impl From<serde_json::Error> for MrgError {
    fn from(value: serde_json::Error) -> Self {
        return MrgError {
            msg: format!("cannot parse JSON: {}", value),
        };
    }
}
impl From<std::io::Error> for MrgError {
    fn from(value: std::io::Error) -> Self {
        return MrgError {
            msg: format!("cannot perform IO: {}", value),
        };
    }
}
