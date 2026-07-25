//! SwissArmyNoife Rust SDK — HTTP admin client (`sak320`).

mod client;
mod error;

pub use client::{
    assert_capacity_ok, assert_egress_ok, assert_list_ok, assert_llm_ok, assert_memory_ok,
    assert_record_ok, assert_research_ok, assert_sandbox_ok, assert_tools_ok, is_egress_miss,
    is_llm_miss, is_memory_miss, is_research_miss, is_sandbox_miss, is_tools_miss,
    node_id_from_broker_record, normalize_claim_work_response, queue_depth_for_session,
    work_session_id, SakClient,
};
pub use error::SdkError;
