use std::{io::{self, Read}, path::Path};
#[cfg(feature = "parser-trace")]
use std::{fs::File, io::{BufRead, BufReader}};

use crate::trace::TraceSession;
#[cfg(feature = "parser-trace")]
use crate::trace::NodeTrace;

#[cfg(feature = "parser-trace")]
#[derive(serde::Deserialize)]
struct TraceJsonV2 {
    trace_version: Option<u32>,
    nodes: Vec<NodeTrace>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceFormat {
    Json,
    Jsonl,
}

#[cfg(not(feature = "parser-trace"))]
pub fn load_trace_file(
    _path: impl AsRef<Path>,
    _format: Option<TraceFormat>,
) -> io::Result<TraceSession> {
    Err(io::Error::other(
        "trace loading requires the `parser-trace` feature",
    ))
}

#[cfg(not(feature = "parser-trace"))]
pub fn detect_trace_format(_path: impl AsRef<Path>) -> io::Result<TraceFormat> {
    Err(io::Error::other(
        "trace loading requires the `parser-trace` feature",
    ))
}

#[cfg(not(feature = "parser-trace"))]
pub fn load_json(_reader: impl Read) -> io::Result<TraceSession> {
    Err(io::Error::other(
        "trace loading requires the `parser-trace` feature",
    ))
}

#[cfg(not(feature = "parser-trace"))]
pub fn load_jsonl(_reader: impl Read) -> io::Result<TraceSession> {
    Err(io::Error::other(
        "trace loading requires the `parser-trace` feature",
    ))
}

#[cfg(feature = "parser-trace")]
pub fn load_trace_file(path: impl AsRef<Path>, format: Option<TraceFormat>) -> io::Result<TraceSession> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let format = match format {
        Some(format) => format,
        None => detect_trace_format(path)?,
    };
    match format {
        TraceFormat::Json => load_json(file),
        TraceFormat::Jsonl => load_jsonl(file),
    }
}

#[cfg(feature = "parser-trace")]
pub fn detect_trace_format(path: impl AsRef<Path>) -> io::Result<TraceFormat> {
    let mut file = File::open(path)?;
    let mut buf = [0_u8; 1];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            return Ok(TraceFormat::Jsonl);
        }
        match buf[0] {
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            b'[' => return Ok(TraceFormat::Json),
            _ => return Ok(TraceFormat::Jsonl),
        }
    }
}

#[cfg(feature = "parser-trace")]
pub fn load_json(reader: impl Read) -> io::Result<TraceSession> {
    let value: serde_json::Value = serde_json::from_reader(reader).map_err(io::Error::other)?;
    if value.is_array() {
        let nodes: Vec<NodeTrace> = serde_json::from_value(value).map_err(io::Error::other)?;
        return Ok(TraceSession::from_events(nodes));
    }
    let payload: TraceJsonV2 = serde_json::from_value(value).map_err(io::Error::other)?;
    let _version = payload.trace_version.unwrap_or(2);
    Ok(TraceSession::from_events(payload.nodes))
}

#[cfg(feature = "parser-trace")]
pub fn load_jsonl(reader: impl Read) -> io::Result<TraceSession> {
    let mut events = Vec::new();
    for line in BufReader::new(reader).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: NodeTrace = serde_json::from_str(&line).map_err(io::Error::other)?;
        events.push(event);
    }
    Ok(TraceSession::from_events(events))
}

