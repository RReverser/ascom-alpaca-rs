#![cfg_attr(
    all(doc, feature = "nightly"),
    feature(doc_auto_cfg, async_fn_in_trait),
    allow(incomplete_features)
)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::as_conversions,
    clippy::clone_on_ref_ptr,
    clippy::default_numeric_fallback,
    clippy::format_push_string,
    clippy::if_then_some_else_none,
    clippy::map_err_ignore,
    clippy::panic_in_result_fn,
    clippy::single_char_lifetime_names,
    clippy::str_to_string,
    clippy::string_to_string,
    clippy::unwrap_used,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    // missing_docs,
    // clippy::cargo,
    noop_method_call,
    single_use_lifetimes,
    unreachable_pub,
    // unsafe_code,
    unused_lifetimes,
    unused_macro_rules,
    unused_qualifications,
    unused_results,
    unused_tuple_struct_fields
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::return_self_not_must_use,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::redundant_pub_crate,
    clippy::single_match_else,
    clippy::type_repetition_in_bounds
)]

pub(crate) mod macros;

#[path = "autogen/mod.rs"]
pub mod api;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

mod discovery;
mod errors;
mod params;
mod response;

pub use api::Devices;
pub use errors::{ASCOMError, ASCOMErrorCode, ASCOMResult};
