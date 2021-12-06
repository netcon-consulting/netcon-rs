//! A collections of tools and helper functions developed for and by NetCon
//! Unternehmensberatung GmbH
//!
//! # Features
//!
//! By default, all features are turned off. To actually get any use out of the `netcon` library,
//! the required features need to be explicitly activated.
//!
//! ## `threadpool`
//!
//! An implementation of a thread pool to run code asynchronously in multiple
//! threads. This enables the [`threadpool`](crate::threadpool) module.

#[cfg(feature = "threadpool")]
pub mod threadpool;
