//! Squirt's social integrations: Last.fm scrobbling and Discord Rich Presence.
//!
//! Squirt's own app registrations are built in (see [`builtin`]), so users
//! only sign in to their own Last.fm account; nobody registers an API app.

pub mod config;
pub mod discord;
pub mod lastfm;

pub use config::SocialConfig;

/// Squirt's app registrations, shared by every user.
///
/// The Last.fm API key and Discord application ID are public identifiers and
/// default to Squirt's own. The Last.fm shared secret is compiled in from the
/// `SQUIRT_LASTFM_API_SECRET` environment variable (set in the git-ignored
/// `.cargo/config.toml` locally, or from a CI secret for releases) so it never
/// sits in the repository. Builds without it have Last.fm sign-in unavailable.
/// Any of the three can be overridden at build time with the env vars below.
pub mod builtin {
    pub const LASTFM_API_KEY: &str = match option_env!("SQUIRT_LASTFM_API_KEY") {
        Some(k) => k,
        None => "bb6e8aca7ca848bbdce32d05a29b56c8",
    };
    pub const LASTFM_API_SECRET: Option<&str> = option_env!("SQUIRT_LASTFM_API_SECRET");
    pub const DISCORD_CLIENT_ID: &str = match option_env!("SQUIRT_DISCORD_CLIENT_ID") {
        Some(id) => id,
        None => "1569215893014052886",
    };
}
