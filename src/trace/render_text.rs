use crate::trace::NodeTrace;

pub fn render_tree(events: &[NodeTrace]) -> String {
    let mut out = String::new();
    for event in events {
        let indent = if event.parent_node_id.is_some() { "  " } else { "" };
        let label = event.label.as_deref().unwrap_or("-");
        let rule = event
            .rule
            .as_ref()
            .map(|rule| {
                let name = rule.rule_name.as_deref().unwrap_or("-");
                format!(
                    " rule#{}({})@{}:{}:{}",
                    rule.rule_id, name, rule.rule_file, rule.rule_line, rule.rule_column
                )
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "{indent}#{} {:?}/{:?} [{}..{}] marker={} {label}{rule}\n",
            event.node_id,
            event.kind,
            event.status,
            event.input_start,
            event.input_end,
            event.is_step_marker
        ));
    }
    out
}

pub fn render_timeline(events: &[NodeTrace]) -> String {
    let mut out = String::new();
    for event in events {
        let label = event.label.as_deref().unwrap_or("-");
        let rule = event
            .rule
            .as_ref()
            .map(|rule| {
                let name = rule.rule_name.as_deref().unwrap_or("-");
                format!(
                    " rule#{}({})@{}:{}:{}",
                    rule.rule_id, name, rule.rule_file, rule.rule_line, rule.rule_column
                )
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "#{:06} p{:?} {:?}/{:?} [{}..{}] marker={} {label}{rule}\n",
            event.node_id,
            event.parent_node_id,
            event.kind,
            event.status,
            event.input_start,
            event.input_end,
            event.is_step_marker
        ));
    }
    out
}

