use crate::error;

pub struct Arguments {
    pub keys: Vec<String>,
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
            clap::Arg::new("files")
                .help("List of files to merge")
                .action(clap::ArgAction::Append),
        )
        .get_matches_from(args);
    let keys = matches
        .get_many::<String>("keys")
        .unwrap()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let paths: Vec<String> = matches
        .get_many::<String>("files")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    return Ok(Arguments { keys, paths });
}
