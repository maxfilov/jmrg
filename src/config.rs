use crate::error;

pub struct Arguments {
    pub ts_keys: Vec<String>,
    pub dt_keys: Vec<String>,
    pub paths: Vec<String>,
}

pub fn parse(args: Vec<String>) -> Result<Arguments, error::MrgError> {
    let matches: clap::ArgMatches = clap::Command::new("jmrg")
        .about("Merges sorted ndjson files into a single sorted stream")
        .arg(
            clap::Arg::new("keys")
                .short('k')
                .help("Specifies keys to look for, can be specified multiple times")
                .default_value("timestamp")
                .action(clap::ArgAction::Append),
        )
        .arg(
            clap::Arg::new("datetime-keys")
                .short('d')
                .help("Specifies iso8601 keys to look for, can be specified multiple times")
                .default_value("datetime")
                .action(clap::ArgAction::Append),
        )
        .arg(
            clap::Arg::new("files")
                .required(true)
                .help("List of files to merge")
                .action(clap::ArgAction::Append),
        )
        .get_matches_from(args);
    let ts_keys = matches
        .get_many::<String>("keys")
        .ok_or(error::MrgError {
            msg: "no 'keys' are provided".to_string(),
        })?
        .map(|s: &String| s.to_string())
        .collect::<Vec<String>>();
    let dt_keys = matches
        .get_many::<String>("datetime-keys")
        .ok_or(error::MrgError {
            msg: "no 'datetime-keys' are provided".to_string(),
        })?
        .map(|s: &String| s.to_string())
        .collect::<Vec<String>>();
    let paths: Vec<String> = matches
        .get_many::<String>("files")
        .ok_or(error::MrgError {
            msg: "no 'files' provided".to_string(),
        })?
        .map(|s: &String| s.to_string())
        .collect::<Vec<String>>();
    Ok(Arguments { ts_keys, dt_keys, paths })
}

#[cfg(test)]
mod tests {
    #[test]
    fn valid_parse() {
        let args = vec![
            "program_name",
            "-k",
            "hello",
            "-k",
            "world",
            "-d",
            "arkady",
            "-d",
            "glinin",
            "1.log",
            "2.log",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
        let parsed = crate::config::parse(args).unwrap();
        assert_eq!(parsed.paths, vec!["1.log", "2.log"]);
        assert_eq!(parsed.ts_keys, vec!["hello", "world"]);
        assert_eq!(parsed.dt_keys, vec!["arkady", "glinin"]);
    }

    #[test]
    fn no_keys() {
        let args = vec!["program_name", "1.log", "2.log"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let parsed = crate::config::parse(args).unwrap();
        assert_eq!(parsed.paths, vec!["1.log", "2.log"]);
        assert_eq!(parsed.ts_keys, vec!["timestamp"]);
        assert_eq!(parsed.dt_keys, vec!["datetime"]);
    }
}
