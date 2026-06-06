//! REST API module for SubJudge contest management system.
//!
//! This module provides HTTP endpoints for managing programming contests, including:
//! - Contest information retrieval and modification
//! - Data synchronization from external sources
//! - Access control and capability management
//! - User authentication and authorization
//! - Administrative functions (account management)
//!
//! # Submodules
//!
//! - [`access`]: Access control API for determining client capabilities
//! - [`admin`]: Administrative endpoints for account and system management
//! - [`auth`]: Authentication endpoints (login, token management)
//! - [`contests`]: Contest management endpoints (retrieve, modify timing)
//! - [`sync`]: Data synchronization endpoints for bulk imports

pub mod access;
pub mod admin;
pub mod auth;
pub mod contests;
pub mod sync;
