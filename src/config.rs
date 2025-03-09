use crate::error;

pub struct Arguments {
    pub ts_keys: Vec<String>,
    pub dt_keys: Vec<String>,
    pub paths: Vec<String>,
}

pub fn parse(args: Vec<String>) -> Result<Arguments, error::MrgError> {
    let matches: clap::ArgMatches = clap::Command::new("jmrg")
        .about(r#"Merges sorted ndjson files into a single sorted stream.

Command accepts two main options: -k and -d. Both of them are used to specify sorting keys
in JSON objects. '-k' is used to specify names of MS-since-Epoch fields and '-d' can be
used to specify names of ISO8601 formatted date time fields. First found always wins and
'-d' has a priority over '-k'"#)
        .arg(
            clap::Arg::new("ts-ms-key")
                .long("ms-key")
                .short('M')
                .help("Specifies keys to look for, can be specified multiple times")
                .default_value("timestamp")
                .action(clap::ArgAction::Append),
        )
        .arg(
            clap::Arg::new("dt-key")
                .long("dt-key")
                .short('D')
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
    Ok(Arguments {
        ts_keys: matches
            .get_many::<String>("ts-ms-key")
            .ok_or(error::MrgError {
                msg: "no 'keys' are provided".to_string(),
            })?
            .map(|s: &String| s.to_string())
            .collect::<Vec<String>>(),
        dt_keys: matches
            .get_many::<String>("dt-key")
            .ok_or(error::MrgError {
                msg: "no 'datetime-keys' are provided".to_string(),
            })?
            .map(|s: &String| s.to_string())
            .collect::<Vec<String>>(),
        paths: matches
            .get_many::<String>("files")
            .ok_or(error::MrgError {
                msg: "no 'files' provided".to_string(),
            })?
            .map(|s: &String| s.to_string())
            .collect::<Vec<String>>(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn valid_parse() {
        let args = vec![
            "program_name",
            "-M",
            "hello",
            "-M",
            "world",
            "-D",
            "arkady",
            "-D",
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
