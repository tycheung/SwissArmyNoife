//! `SwissArmyNoife` `memory.*` helpers (chunk, hash, index, search).

mod chunk;
mod content_hash;
mod embed_offer;
mod fingerprint;
mod hash_embed;
mod hnsw_lite;
mod index_offer;
mod meta_store;
mod naive_index;
mod non_goals;
mod plane;
mod scope;
mod scope_offer;
mod search_offer;
mod vector;

pub use chunk::{chunk_text, Chunk, ChunkParams};
pub use content_hash::content_hash_hex;
pub use embed_offer::MemoryEmbedOffer;
pub use fingerprint::{fingerprint_matches, index_fingerprint};
pub use hash_embed::hash_embed;
pub use hnsw_lite::HnswLite;
pub use index_offer::MemoryIndexOffer;
pub use meta_store::{get_index_fingerprint, upsert_index_meta};
pub use naive_index::{NaiveIndex, SearchHit};
pub use plane::{excerpt, MemoryPlane, MemoryState};
pub use scope::{scope_hash, ScopeKind};
pub use scope_offer::MemoryScopeOffer;
pub use search_offer::MemorySearchOffer;
pub use vector::{BackendKind, DynIndex, VectorIndex};
