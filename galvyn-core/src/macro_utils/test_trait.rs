//! Functions which test whether their argument implements a certain trait
//!
//! They are useful for macros to generate cleaner error messages.
//! For example if a function does not implement `axum`'s `Handler` trait,
//! then its probably because some involved type does not implement some specific trait.
//! The compiler is great at checking but not so great at presenting such relations.
//!
//! To improve this, a macro can simple check all its involved types / values manually by
//! passing them to a functions which explicitly requires a single trait.

use axum::response::IntoResponse;

/// Requires argument to implement `Send`
pub fn send<T: Send>(_t: T) {}

/// Requires argument to implement `IntoResponse`
pub fn into_response<T: IntoResponse>(_t: T) {}
