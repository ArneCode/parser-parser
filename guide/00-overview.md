# Overview

`marser` is a parser-combinator library for writing PEG-style grammars in Rust code.
It focuses on two things:

- expressing grammar rules in a readable, composable style
- producing useful error messages and recoverable results

## What you will learn

In this guide, you will:

- build a small parser with `capture!` and combinators
- learn the difference between `Matcher` and `Parser`
- build a small but complete JSON parser from scratch
- understand error reporting and recovery patterns

## How to use this guide

- If you are new to parser combinators, start with [Quickstart](crate::guide::quickstart).
- If you want the mental model first, read [Core Concepts](crate::guide::core_concepts).
- If you learn best from building a complete grammar, jump to [Build a Simple JSON Parser](crate::guide::worked_json_example).

## Prerequisites

- Rust toolchain installed
- comfort with basic Rust syntax (functions, enums, pattern matching)
- basic command line usage (`cargo run`, `cargo test`)
