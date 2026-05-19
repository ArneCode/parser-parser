//! AI assistance: this file was written with AI assistance. The maintainer reviewed it and did not find errors.

use std::io::{self, Write};

use serde_json::json;

use crate::event::NodeTrace;
use crate::version::SCHEMA_VERSION;

#[derive(Clone, Debug, Default)]
pub struct TraceSession {
    nodes: Vec<NodeTrace>,
    source_text: Option<String>,
    dropped_events: usize,
    max_events: Option<usize>,
}

impl TraceSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_events(max_events: usize) -> Self {
        Self {
            nodes: Vec::new(),
            source_text: None,
            dropped_events: 0,
            max_events: Some(max_events),
        }
    }

    pub fn record(&mut self, node: NodeTrace) {
        if let Some(max_events) = self.max_events
            && self.nodes.len() >= max_events
        {
            self.dropped_events = self.dropped_events.saturating_add(1);
            return;
        }
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[NodeTrace] {
        &self.nodes
    }

    pub fn events(&self) -> &[NodeTrace] {
        self.nodes()
    }

    pub fn dropped_events(&self) -> usize {
        self.dropped_events
    }

    pub fn source_text(&self) -> Option<&str> {
        self.source_text.as_deref()
    }

    pub fn set_source_text(&mut self, source_text: impl Into<String>) {
        self.source_text = Some(source_text.into());
    }

    pub fn write_json<W: Write>(&self, mut writer: W) -> io::Result<()> {
        serde_json::to_writer(
            &mut writer,
            &json!({
                "trace_version": SCHEMA_VERSION,
                "nodes": self.nodes,
                "source_text": self.source_text,
            }),
        )?;
        Ok(())
    }

    pub fn write_jsonl<W: Write>(&self, mut writer: W) -> io::Result<()> {
        for node in &self.nodes {
            serde_json::to_writer(&mut writer, node)?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn from_events(nodes: Vec<NodeTrace>) -> Self {
        Self {
            nodes,
            source_text: None,
            dropped_events: 0,
            max_events: None,
        }
    }
}
