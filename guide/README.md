Book-style guide to `marser` for newcomers and for evaluating fit.

<div style="background-color: #fff8e1; border-left: 4px solid #f9a825; padding: 0.75em 1em; margin: 1em 0;">

**AI assistance:** This chapter was drafted with AI assistance while the library is still young. The guide is expected to improve over time as APIs and examples stabilize. If anything looks wrong or confusing, please [report it on GitHub](https://github.com/ArneCode/marser/issues/new).

</div>

# marser Guide

This guide supports both **newcomers** and **experienced Rust users evaluating** whether `marser` fits their project.

**Policy:** Compatibility, MSRV, and experimental tracing are summarized on the main crate page (the project README, also shown at the top of docs.rs for `marser`).

### If you are evaluating `marser`

Skim in this order:

1. [Overview](crate::guide::overview) — fit, trade-offs, glossary.
2. [Quickstart](crate::guide::quickstart) — dependencies, parser vs matcher, first code.
3. [Errors and Recovery](crate::guide::errors_and_recovery) — `Ok((_, errors))` vs `Err`, `commit_on`, recovery APIs.
4. [Capture and Binds](crate::guide::capture_and_binds) — `capture!`, `bind!` / `bind_span!` / `bind_slice!`.
5. [Worked JSON example](crate::guide::worked_json_example) or the repo `examples/` — full grammar layout.

Then optionally: [Common patterns](crate::guide::common_patterns), [Parser and Matcher Reference](crate::guide::parser_matcher_reference), [Tracing and Debugging](crate::guide::tracing_and_debugging).

### Which page answers which question?

| Question | Best page |
|----------|-----------|
| "Is `marser` a good fit for my parser?" | [Overview](crate::guide::overview) |
| "What does the smallest realistic grammar look like?" | [Quickstart](crate::guide::quickstart) |
| "How do parser and matcher layers divide responsibilities?" | [Core Concepts](crate::guide::core_concepts) |
| "How do I build a complete recursive grammar?" | [Build a Simple JSON Parser](crate::guide::worked_json_example) |
| "How do I get useful diagnostics and partial ASTs?" | [Errors and Recovery](crate::guide::errors_and_recovery) |
| "How do `capture!` and bind shapes actually work?" | [Capture and Binds](crate::guide::capture_and_binds) |
| "What are the common grammar recipes?" | [Common patterns](crate::guide::common_patterns) |
| "Where is the API reference once I know the model?" | [Parser and Matcher Reference](crate::guide::parser_matcher_reference) |
| "When should I enable tracing?" | [Tracing and Debugging](crate::guide::tracing_and_debugging) |

### Full tutorial order

1. [Overview](crate::guide::overview)
2. [Quickstart](crate::guide::quickstart)
3. [Core Concepts](crate::guide::core_concepts)
4. [Build a Simple JSON Parser](crate::guide::worked_json_example)
5. [Errors and Recovery](crate::guide::errors_and_recovery)
6. [Capture and Binds](crate::guide::capture_and_binds)
7. [Common patterns](crate::guide::common_patterns)
8. [Parser and Matcher Reference](crate::guide::parser_matcher_reference)
9. [Tracing and Debugging](crate::guide::tracing_and_debugging)

If you want to run the JSON example from a checkout (matches CI / local workflows):

```bash
cargo run --example json --features annotate-snippets -- tests/data/json1.json
```
