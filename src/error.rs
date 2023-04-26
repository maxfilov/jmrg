use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct MrgError {
    pub msg: String,
}

impl Display for MrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<clap::error::Error> for MrgError {
    fn from(value: clap::error::Error) -> Self {
        MrgError {
            msg: format!("cannot parse command line: {}", value),
        }
    }
}

impl From<serde_json::Error> for MrgError {
    fn from(value: serde_json::Error) -> Self {
        MrgError {
            msg: format!("cannot parse JSON: {}", value),
        }
    }
}

impl From<std::io::Error> for MrgError {
    fn from(value: std::io::Error) -> Self {
        MrgError {
            msg: format!("cannot perform IO: {}", value),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mrg_error_from_std_io_error() {
        let from = std::io::Error::from(std::io::ErrorKind::AddrInUse);
        let mrg_error = crate::error::MrgError::from(from);
        let msg = format!("{}", mrg_error);
        assert_eq!("cannot perform IO: address in use", msg);
    }

    #[test]
    fn mrg_error_from_clap_error() {
        let src = clap::error::Error::new(clap::error::ErrorKind::InvalidSubcommand);
        let mrg_error = crate::error::MrgError::from(src);
        let msg = format!("{}", mrg_error);
        assert_eq!(
            "cannot parse command line: error: unrecognized subcommand\n",
            msg
        );
    }

    #[test]
    fn mrg_error_from_serde_error() {
        let r: serde_json::Result<serde_json::Value> = serde_json::from_str("{asdf");
        let src: serde_json::Error = r.unwrap_err();
        let mrg_error = crate::error::MrgError::from(src);
        let msg = format!("{}", mrg_error);
        assert_eq!(
            "cannot parse JSON: key must be a string at line 1 column 2",
            msg
        );
    }
}
