#![doc = include_str!("../README.md")]
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
    missing_docs,
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
    clippy::type_repetition_in_bounds,
    clippy::let_underscore_untyped
)]

pub(crate) mod macros;

#[path = "api/mod.autogen.rs"]
pub mod api;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "server")]
mod server;

pub mod discovery;
mod either;
mod errors;
mod response;

pub use api::Devices;
#[cfg(feature = "client")]
pub use client::Client;
pub use errors::{ASCOMError, ASCOMErrorCode, ASCOMResult};
#[cfg(feature = "server")]
pub use server::Server;

#[cfg(test)]
#[ctor::ctor]
fn prepare_test_env() {
    use tracing_subscriber::prelude::*;

    std::env::set_var("RUST_BACKTRACE", "full");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("ascom_alpaca", tracing::Level::TRACE),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .with(tracing_error::ErrorLayer::default())
        .init();

    color_eyre::config::HookBuilder::default()
        .add_frame_filter(Box::new(|frames| {
            frames.retain(|frame| {
                frame.filename.as_ref().map_or(false, |filename| {
                    // Only keep our own files in the backtrace to reduce noise.
                    filename.starts_with(env!("CARGO_MANIFEST_DIR"))
                })
            });
        }))
        .install()
        .expect("Failed to install color_eyre");
}
