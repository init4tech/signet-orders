//! Builder binary components.

#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

/// Example to Fill Orders.
pub mod filler;

/// Example to send Orders.
pub mod order;

/// Provider capable of filling and sending transactions.
pub mod provider;

// silence clippy
use chrono as _;
use tokio as _;
