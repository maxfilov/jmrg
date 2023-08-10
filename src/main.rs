use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Lines, Read, Write};

use infer::MatcherType;
use serde::Deserializer;
use serde_json;

mod config;
mod error;

const BUF_SIZE: usize = 1024 * 1024;

///
/// The function attempts to open a file,
/// infers its type (e.g., whether it's an archive like gzip or bzip2),
/// and returns a corresponding Read trait object that can be used to read the file's contents.
/// If the file is not an archive or if it's an unsupported archive, tries to read it as is.
///
/// # Arguments
///
/// * `path`: path to the file in the filesystem
///
/// returns: Result<Box<dyn Read>, MrgError>
///
/// # Examples
///
/// ```
/// let f = open_file("/var/log/vector.log")
/// ```
fn open_file(path: &str) -> Result<Box<dyn Read>, error::MrgError> {
    let file: File = File::open(path)?;
    match infer::get_from_path(path).unwrap() {
        Some(inferred_type) => match inferred_type.matcher_type() {
            MatcherType::Archive => match inferred_type.extension() {
                "gz" => Ok(Box::new(flate2::read::GzDecoder::new(file))),
                "bz2" => Ok(Box::new(bzip2::read::BzDecoder::new(file))),
                // in case it's not archive we know about, we try to parse it as is
                _ => Ok(Box::new(file)),
            },
            // in case it's not archive we try to parse it as is
            _ => Ok(Box::new(file)),
        },
        // in case we couldn't not infer type, we try to parse it as is
        None => Ok(Box::new(file)),
    }
}

fn make_readers(paths: &Vec<String>) -> Result<Vec<BufReader<Box<dyn Read>>>, error::MrgError> {
    Ok(paths
        .into_iter()
        .map(|path| open_file(path))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|s| BufReader::with_capacity(BUF_SIZE, s))
        .collect())
}

struct Source<'a, Input: BufRead> {
    input: Lines<Input>,
    raw_line: String,
    ts: i64,
    keys: &'a HashSet<String>,
}

impl<'a, Input: BufRead> Source<'a, Input> {
    fn new(input: Input, keys: &'a HashSet<String>) -> Option<Self> {
        Self {
            input: input.lines(),
            raw_line: String::new(),
            ts: -1,
            keys,
        }
        .fetch_next()
    }

    fn fetch_next(mut self) -> Option<Self> {
        while let Some(next_line) = self.input.next() {
            match next_line {
                Ok(raw_line) => {
                    let mut des = serde_json::de::Deserializer::from_str(raw_line.as_str());
                    match des.deserialize_map(EntryVisitor { keys: self.keys }) {
                        Ok(ts) => {
                            self.ts = ts;
                            self.raw_line = raw_line;
                            return Some(self);
                        }
                        Err(e) => {
                            eprintln!("cannot parse entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("cannot get next line: {}", e);
                }
            }
        }
        None
    }
}

impl<'a, T: BufRead> Eq for Source<'a, T> {}

impl<'a, T: BufRead> PartialEq<Self> for Source<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ts == other.ts
    }
}

impl<'a, T: BufRead> PartialOrd<Self> for Source<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.ts.cmp(&self.ts))
    }
}

impl<T: BufRead> Ord for Source<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.ts.cmp(&self.ts)
    }
}

struct EntryVisitor<'a> {
    keys: &'a HashSet<String>,
}

impl<'de> serde::de::Visitor<'de> for EntryVisitor<'de> {
    type Value = i64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "map with keys from provided set")
    }

    #[inline]
    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut ts: Option<i64> = None;

        while let Some(k) = map.next_key::<&str>()? {
            if ts.is_none() && self.keys.contains(k) {
                ts = Some(map.next_value::<i64>()?);
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }

        ts.ok_or(serde::de::Error::custom("no fields of the provided set"))
    }
}

pub fn run<Input: BufRead, Output: Write>(
    keys: Vec<String>,
    ins: Vec<Input>,
    out: &mut Output,
) -> Result<(), error::MrgError> {
    // global semi-constants initialization
    let key_set: HashSet<String> = HashSet::from_iter(keys.into_iter());
    let mut sources: BinaryHeap<Source<Input>> = ins
        .into_iter()
        .filter_map(|input: Input| Source::new(input, &key_set))
        .collect();
    while !sources.is_empty() {
        let source: Source<Input> = sources.pop().unwrap();
        writeln!(out, "{}", source.raw_line.as_str())?;
        if let Some(s) = source.fetch_next() {
            sources.push(s);
        }
    }
    Ok(())
}

fn main() -> Result<(), error::MrgError> {
    let cmd_args: Vec<String> = env::args().collect();
    let args: config::Arguments = config::parse(cmd_args)?;

    let sources: Vec<BufReader<Box<dyn Read>>> = make_readers(&args.paths)?;
    let mut output = BufWriter::with_capacity(BUF_SIZE, std::io::stdout());
    run(args.keys, sources, &mut output)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};

    #[test]
    fn normal_run() {
        let keys = vec![String::from("t")];
        let in1 = BufReader::new(stringreader::StringReader::new(
            r#"
{"t":15, "add": "15_1"}
{"t":16, "add": "16_1"}
{"t":18, "add": "18_1"}
"#,
        ));
        let in2 = BufReader::new(stringreader::StringReader::new(
            r#"
{"t":16, "add": "16_2"}
{"t":17, "add": "17_2"}
{"t":18, "add": "18_2"}
"#,
        ));
        let mut buf = std::io::BufWriter::new(Vec::new());
        crate::run(keys, vec![in1, in2], &mut buf).unwrap();
        let result = String::from_utf8(buf.into_inner().unwrap()).unwrap();
        assert_eq!(
            r#"{"t":15, "add": "15_1"}
{"t":16, "add": "16_2"}
{"t":16, "add": "16_1"}
{"t":17, "add": "17_2"}
{"t":18, "add": "18_1"}
{"t":18, "add": "18_2"}
"#,
            result
        );
    }

    #[test]
    fn open_file() {
        let mut r = BufReader::with_capacity(
            1024,
            crate::open_file(&String::from("tests/data/1.json")).unwrap(),
        );
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        let replaced = line.replace("\r", "").replace("\n", "");
        assert_eq!(r#"{"t":15, "add": "15_1"}"#, replaced);
    }
}
