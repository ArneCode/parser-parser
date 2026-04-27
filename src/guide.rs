#![doc = include_str!("../guide/README.md")]

//! Additional guide pages:
//!
//! - [`overview`]
//! - [`quickstart`]
//! - [`core_concepts`]
//! - [`worked_json_example`]
//! - [`errors_and_recovery`]
//! - [`capture_and_binds`]
//! - [`parser_matcher_reference`]

pub mod overview {
    #![doc = include_str!("../guide/00-overview.md")]
}

pub mod quickstart {
    #![doc = include_str!("../guide/01-quickstart.md")]
}

pub mod core_concepts {
    #![doc = include_str!("../guide/02-core-concepts.md")]
}

pub mod worked_json_example {
    #![doc = include_str!("../guide/03-worked-json-example.md")]
}

pub mod errors_and_recovery {
    #![doc = include_str!("../guide/04-errors-and-recovery.md")]
}

pub mod capture_and_binds {
    #![doc = include_str!("../guide/05-capture-and-binds.md")]
}

pub mod parser_matcher_reference {
    #![doc = include_str!("../guide/06-parser-matcher-reference.md")]
}
