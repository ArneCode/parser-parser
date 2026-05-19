//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

use serde::Deserialize;

use crate::event::NodeTrace;
use crate::session::TraceSession;
use crate::version::check_trace_version;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceFormat {
    Json,
    Jsonl,
}

#[derive(Deserialize)]
struct TraceJsonEnvelope {
    trace_version: Option<u32>,
    nodes: Vec<NodeTrace>,
    source_text: Option<String>,
}

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
            b'[' | b'{' => return Ok(TraceFormat::Json),
            _ => return Ok(TraceFormat::Jsonl),
        }
    }
}

pub fn load_json(reader: impl Read) -> io::Result<TraceSession> {
    let value: serde_json::Value = serde_json::from_reader(reader).map_err(io::Error::other)?;
    if value.is_array() {
        let nodes: Vec<NodeTrace> = serde_json::from_value(value).map_err(io::Error::other)?;
        return Ok(TraceSession::from_events(nodes));
    }
    let payload: TraceJsonEnvelope = serde_json::from_value(value).map_err(io::Error::other)?;
    check_trace_version(payload.trace_version).map_err(|e| io::Error::other(e.to_string()))?;
    let mut session = TraceSession::from_events(payload.nodes);
    if let Some(source_text) = payload.source_text {
        session.set_source_text(source_text);
    }
    Ok(session)
}

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
