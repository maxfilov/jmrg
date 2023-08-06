use std::cell::RefCell;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Lines, Read, Write};

use infer::MatcherType;
use serde_json;

mod config;
mod error;

thread_local!(
    static KEYS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static KEYS_STR: RefCell<String> = RefCell::new(String::new());
);

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
    let mut readers: Vec<BufReader<Box<dyn Read>>> = vec![];
    for path in paths {
        let reader = BufReader::with_capacity(BUF_SIZE, open_file(path)?);
        readers.push(reader);
    }
    Ok(readers)
}

struct Source<T: BufRead> {
    it: Lines<T>,
    value: String,
    ts: i64,
}

impl<T: BufRead> Source<T> {
    fn new(s: T) -> Option<Self> {
        Source {
            it: s.lines(),
            value: String::new(),
            ts: 0,
        }
        .fetch_next()
    }

    fn fetch_next(mut self) -> Option<Self> {
        while let Some(next_line) = self.it.next() {
            match next_line {
                Ok(value) => match serde_json::from_str::<Entry>(value.as_str()) {
                    Ok(entry) => {
                        self.ts = entry.key;
                        self.value = value;
                        return Some(self);
                    }
                    Err(e) => {
                        eprintln!("cannot parse entry: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("cannot get next line: {}", e);
                }
            }
        }
        None
    }
}

impl<T: BufRead> Eq for Source<T> {}

impl<T: BufRead> PartialEq<Self> for Source<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ts == other.ts
    }
}

impl<T: BufRead> PartialOrd<Self> for Source<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.ts.cmp(&self.ts))
    }
}

impl<T: BufRead> Ord for Source<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.ts.cmp(&self.ts)
    }
}

struct EntryVisitor;

impl<'de> serde::de::Visitor<'de> for EntryVisitor {
    type Value = Entry;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        KEYS_STR.with(|s: &RefCell<String>| {
            write!(formatter, "map with keys from set '{}'", s.borrow())
        })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut ts: Option<i64> = None;

        while let Some(k) = map.next_key::<&str>()? {
            if ts.is_none() && KEYS.with(|s| s.borrow().contains(k)) {
                ts = Some(map.next_value::<i64>()?);
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }

        match ts {
            Some(val) => Ok(Entry { key: val }),
            None => KEYS_STR.with(|s: &RefCell<String>| {
                Err(serde::de::Error::custom(format!(
                    "no fields of set '{}'",
                    s.borrow()
                )))
            }),
        }
    }
}

struct Entry {
    key: i64,
}

impl<'de> serde::de::Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_map(EntryVisitor)
    }
}

pub fn run<Input: BufRead, Output: Write>(
    keys: Vec<String>,
    ins: Vec<Input>,
    out: &mut Output,
) -> Result<(), error::MrgError> {
    // global semi-constants initialization
    KEYS.with(|s: &RefCell<HashSet<String>>| s.borrow_mut().extend(keys));
    KEYS_STR.with(|keys_str: &RefCell<String>| {
        keys_str.borrow_mut().push_str(
            KEYS.with(|s: &RefCell<HashSet<String>>| {
                s.borrow()
                    .iter()
                    .map(|x: &String| x.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            })
            .as_str(),
        );
    });
    let mut sources: BinaryHeap<Source<Input>> = ins
        .into_iter()
        .filter_map(|input: Input| Source::new(input))
        .collect();
    while !sources.is_empty() {
        let source: Source<Input> = sources.pop().unwrap();
        writeln!(out, "{}", source.value.as_str())?;
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
