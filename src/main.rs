use std::cell::RefCell;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Lines, Read, Write};

use serde_json;
use infer::{MatcherType, Type};

mod config;
mod error;

thread_local!(
    static KEYS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static KEYS_STR: RefCell<String> = RefCell::new(String::new());
);

const BUF_SIZE: usize = 1024 * 1024;

fn open_file(path: &String, file: File) -> Result<Box<dyn Read>, error::MrgError> {
    let maybe_inferred_type: Option<Type> = infer::get_from_path(path).unwrap();

    if None == maybe_inferred_type {
        return Ok(Box::new(file));
    }

    let inferred_type = maybe_inferred_type.unwrap();
    if inferred_type.matcher_type() != MatcherType::Archive {
        return Ok(Box::new(file));
    }

    let extension = inferred_type.extension();
    match extension {
        "gz" => Ok(Box::new(flate2::read::GzDecoder::new(file))),
        "bz2" => Ok(Box::new(bzip2::read::BzDecoder::new(file))),
        _ => Err(error::MrgError {
            msg: format!("cannot open archive of type {}", extension),
        }),
    }
}

fn make_reader(path: &String) -> Result<BufReader<Box<dyn Read>>, error::MrgError> {
    let file: File = File::open(path)?;
    Ok(BufReader::with_capacity(BUF_SIZE, open_file(path, file)?))
}

fn make_readers(paths: &Vec<String>) -> Result<Vec<BufReader<Box<dyn Read>>>, error::MrgError> {
    let mut readers: Vec<BufReader<Box<dyn Read>>> = vec![];
    for path in paths {
        readers.push(make_reader(path)?);
    }
    Ok(readers)
}

struct Source<T: BufRead> {
    it: Lines<T>,
    value: Option<String>,
    ts: Option<i64>,
}

impl<T: BufRead> Source<T> {
    fn new(s: T) -> Self {
        let source = Source {
            it: s.lines(),
            value: None,
            ts: None,
        };
        source.fetch_next()
    }

    fn has_value(&self) -> bool {
        self.ts.is_some()
    }

    fn fetch_next(mut self) -> Self {
        loop {
            let maybe_next_line: Option<std::io::Result<String>> = self.it.next();
            if maybe_next_line.is_none() {
                self.ts = None;
                self.value = None;
                break;
            }
            let next_line: std::io::Result<String> = unsafe { maybe_next_line.unwrap_unchecked() };
            if next_line.is_err() {
                eprintln!("cannot get next line: {}", unsafe {
                    next_line.unwrap_err_unchecked()
                });
                continue;
            }
            let value: String = unsafe { next_line.unwrap_unchecked() };
            let parsed_entry: serde_json::Result<Entry> = serde_json::from_str(value.as_str());
            if parsed_entry.is_err() {
                eprintln!("cannot parse entry: {}", unsafe {
                    parsed_entry.unwrap_err_unchecked()
                });
                continue;
            }
            let entry: Entry = unsafe { parsed_entry.unwrap_unchecked() };
            self.ts = Some(entry.key);
            self.value = Some(value);
            break;
        }
        self
    }
}

impl<T: BufRead> Eq for Source<T> {}

impl<T: BufRead> PartialEq<Self> for Source<T> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.ts.unwrap_unchecked() == other.ts.unwrap_unchecked() }
    }
}

impl<T: BufRead> PartialOrd<Self> for Source<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        unsafe { Some(other.ts.unwrap_unchecked().cmp(&self.ts.unwrap_unchecked())) }
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
            if KEYS.with(|s| s.borrow().contains(k)) && ts.is_none() {
                ts = Some(map.next_value::<i64>()?);
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }

        match ts {
            Some(val) => Ok(Entry { key: val }),
            None => KEYS_STR.with(|s: &RefCell<String>| {
                Err(serde::de::Error::custom(format!(
                    "missing one the fields of set '{}'",
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
    // global semi-contants initialization
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
        .map(|input: Input| Source::new(input))
        .filter(|s: &Source<Input>| s.has_value())
        .collect();
    while !sources.is_empty() {
        let mut source: Source<Input> = unsafe { sources.pop().unwrap_unchecked() };
        writeln!(out, "{}", source.value.as_ref().unwrap().as_str())?;
        source = source.fetch_next();
        if !source.has_value() {
            continue;
        }
        sources.push(source);
    }
    Ok(())
}

fn main() -> Result<(), error::MrgError> {
    let cmd_args: Vec<String> = env::args().collect();
    let maybe_args: Result<config::Arguments, error::MrgError> = config::parse(cmd_args);
    if maybe_args.is_err() {
        eprintln!("{}", unsafe { maybe_args.unwrap_err_unchecked() });
        eprintln!("command arguments are invalid, run with '-h' to see usage");
        std::process::exit(1);
    }
    let args: config::Arguments = maybe_args.unwrap();

    let sources: Vec<BufReader<Box<dyn Read>>> = make_readers(&args.paths)?;
    let mut output: BufWriter<std::io::Stdout> =
        BufWriter::with_capacity(BUF_SIZE, std::io::stdout());
    run(args.keys, sources, &mut output)
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;
    #[test]
    fn normal_run() {
        let keys = vec![String::from("t")];
        let in1 = std::io::BufReader::new(stringreader::StringReader::new(
            r#"
{"t":15, "add": "15_1"}
{"t":16, "add": "16_1"}
{"t":18, "add": "18_1"}
"#,
        ));
        let in2 = std::io::BufReader::new(stringreader::StringReader::new(
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
        let mut r = crate::make_reader(&String::from("tests/data/1.json")).unwrap();
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        let replaced = line.replace("\r", "").replace("\n", "");
        assert_eq!(r#"{"t":15, "add": "15_1"}"#, replaced);
    }
}
