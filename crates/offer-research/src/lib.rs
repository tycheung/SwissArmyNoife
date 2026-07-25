//! `SwissArmyNoife` `research.*` helpers (fetch, sanitize, brief).

mod brief_offer;
mod brief_store;
mod fetch;
mod fetch_offer;
mod patterns;
mod sanitize;

pub use brief_offer::ResearchBriefOffer;
pub use brief_store::{get_brief, list_briefs, put_brief, Brief};
pub use fetch::{research_fetch, ResearchBody};
pub use fetch_offer::ResearchFetchOffer;
pub use patterns::{extract_domains, extract_urls, keyword_hits};
pub use sanitize::sanitize_untrusted;
