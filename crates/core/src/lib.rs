//! Portable application logic, read-only protocol and presentation contracts.
//!
//! This crate must stay independent of GUI toolkits and platform APIs. Hardware
//! access and settings persistence belong to application/platform adapters.
#![forbid(unsafe_code)]

pub mod application;
pub mod battery_service;
pub mod i18n;
pub mod model;
pub mod presentation;
pub mod protocol;
pub mod settings;
