//! Seed search engine.
//!
//! [`generator`] wraps cubiomes, [`version`] maps Minecraft releases to
//! cubiomes enums, [`catalog`] feeds the rule builder, and [`rules`] +
//! [`seed_iter`] implement the search itself.

mod ffi;

pub mod anchors;
pub mod catalog;
pub mod generator;
pub mod rules;
pub mod seed_iter;
pub mod version;
