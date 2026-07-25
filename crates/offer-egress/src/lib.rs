//! `SwissArmyNoife` `network.egress` helpers and offers.

mod allowlist;
mod body_cap;
mod byte_cap;
mod check_offer;
mod fetch;
mod fetch_offer;
mod policy;
mod principal;
mod url_host;

pub use allowlist::HostnameAllowlist;
pub use body_cap::{collect_capped, enforce_response_bytes};
pub use byte_cap::ResponseByteCap;
pub use check_offer::EgressCheckOffer;
pub use fetch::{guarded_get, GuardedBody, HttpGet, ReqwestGet};
pub use fetch_offer::EgressFetchOffer;
pub use policy::EgressPolicy;
pub use principal::PrincipalAllowlist;
pub use url_host::{check_url, host_from_url};
