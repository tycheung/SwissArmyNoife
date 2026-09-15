//! `SwissArmyNoife` `browser.*` — process-sidecar page session (ADR 013).

mod driver;
mod playwright;
mod session_offer;
mod stub;

pub use driver::{BrowserBackend, BrowserBackendKind};
pub use playwright::PlaywrightBackend;
pub use session_offer::{BrowserSessionOffer, BROWSER_BACKEND};
pub use stub::StubBackend;
