//! REST API module for SubJudge contest management system.
//!
//! This module provides HTTP endpoints for managing programming contests, including:
//! - Contest information retrieval and modification
//! - Data synchronization from external sources
//! - Access control and capability management
//!
//! # Submodules
//!
//! - [`access`]: Access control API for determining client capabilities
//! - [`contests`]: Contest management endpoints (retrieve, modify timing)
//! - [`sync`]: Data synchronization endpoints for bulk imports

pub mod access;
pub mod contests;
pub mod sync;
