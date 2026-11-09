//! Squirt's social integrations: Last.fm scrobbling and Discord Rich Presence.
//!
//! Neither service can be used without an app registration, which each user
//! (or a Squirt distributor) provides: a Last.fm API key and secret, and a
//! Discord application ID. They are stored with the rest of the settings in
//! [`SocialConfig`].

pub mod config;
pub mod discord;
pub mod lastfm;

pub use config::SocialConfig;
