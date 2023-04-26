use serde_json;
use std::cell::RefCell;
use std::collections::{BinaryHeap, HashSet};
use std::env;
mod config;
mod error;
use infer::{MatcherType, Type};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Lines, Read, Write};

thread_local!(
    static KEYS: RefCell<HashSet<String>> = RefCell::new(HashSet::new())
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

struct Source {
    it: Lines<BufReader<Box<dyn Read>>>,
    value: Option<String>,
    ts: Option<i64>,
}

impl Source {
    fn new(s: BufReader<Box<dyn Read>>) -> Source {
        let mut source = Source {
            it: s.lines(),
            value: None,
            ts: None,
        };
        loop {
            match source.fetch_next() {
                Ok(_) => break,
                Err(_) => continue,
            };
        }
        source
    }

    fn has_value(&self) -> bool {
        self.ts.is_some()
    }

    fn fetch_next(&mut self) -> Result<(), error::MrgError> {
        let n = self.it.next();
        if n.is_none() {
            self.ts = None;
        } else {
            let value = n.unwrap().unwrap();
            let event: Event = serde_json::from_str(value.as_str())?;
            self.ts = Some(event.timestamp);
            self.value = Some(value);
        }
        Ok(())
    }
}

impl Eq for Source {}

impl PartialEq<Self> for Source {
    fn eq(&self, other: &Self) -> bool {
        self.ts.unwrap() == other.ts.unwrap()
    }
}

impl PartialOrd<Self> for Source {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.ts.unwrap().cmp(&self.ts.unwrap()))
    }
}

impl Ord for Source {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.ts.cmp(&self.ts)
    }
}

struct EventVisitor;

impl<'de> serde::de::Visitor<'de> for EventVisitor {
    type Value = Event;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "map with keys from set '{}'",
            KEYS.with(|s| {
                s.borrow()
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            })
        )
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
            Some(val) => Ok(Event { timestamp: val }),
            None => Err(serde::de::Error::custom(format!(
                "missing one the fields of set '{}'",
                KEYS.with(|s| {
                    s.borrow()
                        .iter()
                        .map(|x| x.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                })
            ))),
        }
    }
}

struct Event {
    timestamp: i64,
}

impl<'de> serde::de::Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_map(EventVisitor)
    }
}

pub fn run<T: Write>(
    ins: Vec<BufReader<Box<dyn Read>>>,
    mut out: BufWriter<T>,
) -> Result<(), error::MrgError> {
    let mut sources: BinaryHeap<Source> = ins
        .into_iter()
        .map(|buf_reader| Source::new(buf_reader))
        .filter(|s| s.has_value())
        .collect();
    while !sources.is_empty() {
        let mut source: Source = sources.pop().unwrap();
        writeln!(out, "{}", source.value.as_ref().unwrap().as_str())?;
        source.fetch_next()?;
        if !source.has_value() {
            continue;
        }
        sources.push(source);
    }
    Ok(())
}

fn main() -> Result<(), error::MrgError> {
    let args = config::parse(env::args().collect::<Vec<String>>())?;
    KEYS.with(|s| s.borrow_mut().extend(args.keys));

    let sources: Vec<BufReader<Box<dyn Read>>> = make_readers(&args.paths)?;
    run(
        sources,
        BufWriter::with_capacity(BUF_SIZE, std::io::stdout()),
    )
}
