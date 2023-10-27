// #![deny(unsafe_code)]
// #![warn(
//     clippy::dbg_macro,
//     clippy::todo,
//     clippy::unimplemented,
//     clippy::print_stdout,
//     clippy::clone_on_copy
// )]
// #![allow(unused_qualifications, unused_imports)]

#[macro_use]
pub extern crate tracing;

pub mod configuration;
pub mod errors;
pub mod modules;
pub mod routes;
pub mod state;
pub mod utils;
