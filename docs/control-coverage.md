# Control crate coverage report (sak063-d)

Generated: 2026-07-23 19:26:40

Target: **>=85% line coverage** on the `control` crate.

**sak063 status: measured** - line coverage >=85% on control crate.

## Tool

- Used: `cargo-llvm-cov`
- Command: see Measuring coverage below

## Summary

```
cargo.exe : info: cargo-llvm-cov currently setting cfg(coverage); you can opt-out it by passing --no-cfg-coverage
At D:\Agentic\SwissArmyNoife\scripts\control_coverage.ps1:12 char:12
+     $out = & cargo @CargoArgs 2>&1 | Out-String
+            ~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: (info: cargo-llv...no-cfg-coverage:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
   Compiling control v0.1.0 (D:\Agentic\SwissArmyNoife\crates\control)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.17s
     Running unittests src\lib.rs (target-alt\llvm-cov-target\debug\deps\control-e9913666423c47a2.exe)

running 58 tests
test api_key::tests::mint_verify_roundtrip ... ok
test budget::tests::token_cap_exhausts ... ok
test api_key::tests::export_load_roundtrip ... ok
test binding::tests::purge_expired_removes_stale_rows ... ok
test api_key::tests::secret_not_stored_plaintext ... ok
test audit::tests::redact_json_masks_secret_keys_nested ... ok
test budget::tests::bytes_and_wall_caps_independent ... ok
test audit::tests::audit_log_records_redacted_invoke ... ok
test binding::tests::zero_ttl_is_immediately_expired ... ok
test binding::tests::api_key_principal_roundtrip ... ok
test audit::tests::soft_delete_hides_from_list_active ... ok
test audit::tests::purge_before_removes_old_soft_deleted_only ... ok
test budget::tests::unlimited_accepts_large_charges ... ok
test binding::tests::bind_get_unbind_roundtrip ... ok
test catalog::tests::get_missing_is_offer_not_found ... ok
test dispatch::tests::dispatch_happy_path_echoes_args ... ok
test dispatch::tests::dispatch_rejects_offer_mismatch ... ok
test dispatch::tests::dispatch_rejects_missing_or_unbound ... ok
test dispatch::tests::dispatch_rejects_policy_denied ... ok
test health::tests::empty_snapshot_defaults ... ok
test catalog::tests::register_list_get_roundtrip ... ok
test idempotency::tests::provision_conflicting_fingerprint_is_schema_invalid ... ok
test catalog::tests::register_replaces_same_id ... ok
test idempotency::tests::conflicting_fingerprint_is_schema_invalid ... ok
test offer::tests::mock_echo_offer_invoke_returns_args ... ok
test idempotency::tests::expired_entry_is_purged_on_lookup ... ok
test idempotency::tests::provision_replay_same_fingerprint_returns_resource_id ... ok
test health::tests::health_ok ... ok
test meter::tests::jsonl_values_match_snapshot ... ok
test idempotency::tests::replay_same_fingerprint_returns_binding_id ... ok
test health::tests::empty_offer_snapshot_json_matches_provider ... ok
test policy::tests::ambient_allows_any_pair ... ok
test meter::tests::jsonl_has_three_metrics ... ok
test policy::tests::allowlist_denies_until_granted ... ok
test policy::tests::grant_is_offer_specific ... ok
test policy_templates::tests::both_set_is_invalid ... ok
test policy_templates::tests::inline_only_returns_copy ... ok
test policy_templates::tests::list_template_names_matches_builtins ... ok
test policy_templates::tests::local_dev_is_empty_object ... ok
test policy_templates::tests::offline_denies_network ... ok
test policy_templates::tests::strict_egress_blocks_hosts ... ok
test policy_templates::tests::unknown_template_is_invalid ... ok
test principal::tests::from_bind_arg_normalizes ... ok
test principal::tests::local_and_api_key_kinds ... ok
test provision::tests::get_missing_is_schema_invalid ... ok
test provision::tests::mark_failed_from_ready ... ok
test provision::tests::provision_ready_then_release ... ok
test rate_limit::tests::burst_exhaust_denies ... ok
test rate_limit::tests::deny_message_is_stable ... ok
test rate_limit::tests::principals_are_isolated ... ok
test rate_limit::tests::unlimited_never_denies ... ok
test risk::tests::from_policy_reads_risk_caps ... ok
test risk::tests::missing_risk_caps_is_unlimited ... ok
test risk::tests::tool_and_shell_caps_exhaust ... ok
test risk::tests::write_bytes_cap ... ok
test trace::tests::invoke_span_records_correlation_fields ... ok
test dispatch::tests::dispatch_rejects_ttl_expired_binding ... ok
test rate_limit::tests::refill_restores_after_burst_exhausted ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s

     Running tests\coverage_smoke.rs (target-alt\llvm-cov-target\debug\deps\coverage_smoke-f871e51e319d7189.exe)

running 12 tests
test smoke_api_key_mint_get_and_export ... ok
test smoke_meter_jsonl_non_empty ... ok
test smoke_rate_limit_denies_after_burst ... ok
test smoke_policy_template_mutual_exclusion ... ok
test smoke_empty_health_snapshot_shape ... ok
test smoke_policy_template_strict_egress_happy_path ... ok
test smoke_idempotency_conflict_is_schema_invalid ... ok
test smoke_idempotency_provision_namespace ... ok
test smoke_unknown_policy_template_is_invalid ... ok
test smoke_broker_health_catalog_version ... ok
test smoke_purge_before_soft_deleted ... ok
test smoke_soft_delete_hides_from_active_list ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
api_key.rs                        177                 6    96.61%          12                 0   100.00%         100                 0   100.00%           0                 0         -
audit.rs                          282                10    96.45%          20                 1    95.00%         167                 8    95.21%           0                 0         -
binding.rs                        192                11    94.27%          17                 2    88.24%         106                 8    92.45%           0                 0         -
budget.rs                         115                 3    97.39%          11                 1    90.91%          68                 3    95.59%           0                 0         -
catalog.rs                        116                 6    94.83%          11                 1    90.91%          50                 3    94.00%           0                 0         -
dispatch.rs                       355                35    90.14%          27                 8    70.37%         280                32    88.57%           0                 0         -
health.rs                          91                19    79.12%          19                 8    57.89%          71                17    76.06%           0                 0         -
idempotency.rs                    291                12    95.88%          16                 0   100.00%         153                 8    94.77%           0                 0         -
meter.rs                           56                 0   100.00%           5                 0   100.00%          37                 0   100.00%           0                 0         -
offer.rs                           87                 4    95.40%          15                 0   100.00%          64                 2    96.88%           0                 0         -
policy.rs                         108                 0   100.00%           9                 0   100.00%          51                 0   100.00%           0                 0         -
policy_templates.rs                87                 1    98.85%          10                 0   100.00%          49                 1    97.96%           0                 0         -
principal.rs                       71                 4    94.37%           8                 1    87.50%          54                 4    92.59%           0                 0         -
provision.rs                      136                15    88.97%          11                 2    81.82%          84                10    88.10%           0                 0         -
rate_limit.rs                     122                14    88.52%          11                 2    81.82%          84                16    80.95%           0                 0         -
risk.rs                           128                 7    94.53%          14                 2    85.71%          88                 6    93.18%           0                 0         -
trace.rs                           83                 7    91.57%          11                 1    90.91%          54                 3    94.44%           0                 0         -
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
TOTAL                            2497               154    93.83%         227                29    87.22%        1560               121    92.24%           0                 0         -
```

## Test run (fallback)

```
cargo.exe :     Finished `test` profile [unoptimized + debuginfo] target(s) in 0.12s
At D:\Agentic\SwissArmyNoife\scripts\control_coverage.ps1:12 char:12
+     $out = & cargo @CargoArgs 2>&1 | Out-String
+            ~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: (    Finished `t...get(s) in 0.12s:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
     Running unittests src\lib.rs (target-alt\debug\deps\control-e9913666423c47a2.exe)

running 58 tests
test audit::tests::purge_before_removes_old_soft_deleted_only ... ok
test api_key::tests::export_load_roundtrip ... ok
test audit::tests::redact_json_masks_secret_keys_nested ... ok
test audit::tests::audit_log_records_redacted_invoke ... ok
test api_key::tests::secret_not_stored_plaintext ... ok
test binding::tests::purge_expired_removes_stale_rows ... ok
test audit::tests::soft_delete_hides_from_list_active ... ok
test api_key::tests::mint_verify_roundtrip ... ok
test binding::tests::bind_get_unbind_roundtrip ... ok
test binding::tests::api_key_principal_roundtrip ... ok
test binding::tests::zero_ttl_is_immediately_expired ... ok
test budget::tests::bytes_and_wall_caps_independent ... ok
test budget::tests::token_cap_exhausts ... ok
test catalog::tests::get_missing_is_offer_not_found ... ok
test budget::tests::unlimited_accepts_large_charges ... ok
test catalog::tests::register_list_get_roundtrip ... ok
test catalog::tests::register_replaces_same_id ... ok
test dispatch::tests::dispatch_rejects_missing_or_unbound ... ok
test dispatch::tests::dispatch_happy_path_echoes_args ... ok
test dispatch::tests::dispatch_rejects_offer_mismatch ... ok
test health::tests::empty_offer_snapshot_json_matches_provider ... ok
test dispatch::tests::dispatch_rejects_policy_denied ... ok
test health::tests::empty_snapshot_defaults ... ok
test health::tests::health_ok ... ok
test idempotency::tests::expired_entry_is_purged_on_lookup ... ok
test idempotency::tests::conflicting_fingerprint_is_schema_invalid ... ok
test idempotency::tests::provision_conflicting_fingerprint_is_schema_invalid ... ok
test idempotency::tests::replay_same_fingerprint_returns_binding_id ... ok
test idempotency::tests::provision_replay_same_fingerprint_returns_resource_id ... ok
test meter::tests::jsonl_has_three_metrics ... ok
test meter::tests::jsonl_values_match_snapshot ... ok
test offer::tests::mock_echo_offer_invoke_returns_args ... ok
test policy::tests::allowlist_denies_until_granted ... ok
test policy::tests::ambient_allows_any_pair ... ok
test policy::tests::grant_is_offer_specific ... ok
test policy_templates::tests::both_set_is_invalid ... ok
test policy_templates::tests::inline_only_returns_copy ... ok
test policy_templates::tests::list_template_names_matches_builtins ... ok
test policy_templates::tests::local_dev_is_empty_object ... ok
test policy_templates::tests::offline_denies_network ... ok
test policy_templates::tests::strict_egress_blocks_hosts ... ok
test policy_templates::tests::unknown_template_is_invalid ... ok
test principal::tests::from_bind_arg_normalizes ... ok
test principal::tests::local_and_api_key_kinds ... ok
test provision::tests::get_missing_is_schema_invalid ... ok
test provision::tests::mark_failed_from_ready ... ok
test provision::tests::provision_ready_then_release ... ok
test rate_limit::tests::burst_exhaust_denies ... ok
test rate_limit::tests::deny_message_is_stable ... ok
test rate_limit::tests::principals_are_isolated ... ok
test rate_limit::tests::unlimited_never_denies ... ok
test risk::tests::from_policy_reads_risk_caps ... ok
test risk::tests::missing_risk_caps_is_unlimited ... ok
test risk::tests::tool_and_shell_caps_exhaust ... ok
test risk::tests::write_bytes_cap ... ok
test trace::tests::invoke_span_records_correlation_fields ... ok
test dispatch::tests::dispatch_rejects_ttl_expired_binding ... ok
test rate_limit::tests::refill_restores_after_burst_exhausted ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s

     Running tests\coverage_smoke.rs (target-alt\debug\deps\coverage_smoke-f871e51e319d7189.exe)

running 12 tests
test smoke_empty_health_snapshot_shape ... ok
test smoke_policy_template_mutual_exclusion ... ok
test smoke_unknown_policy_template_is_invalid ... ok
test smoke_idempotency_conflict_is_schema_invalid ... ok
test smoke_broker_health_catalog_version ... ok
test smoke_policy_template_strict_egress_happy_path ... ok
test smoke_api_key_mint_get_and_export ... ok
test smoke_purge_before_soft_deleted ... ok
test smoke_rate_limit_denies_after_burst ... ok
test smoke_idempotency_provision_namespace ... ok
test smoke_meter_jsonl_non_empty ... ok
test smoke_soft_delete_hides_from_active_list ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Measuring coverage

Install one of:

    cargo install cargo-llvm-cov
    cargo install cargo-tarpaulin

From SwissArmyNoife/:

    cargo llvm-cov -p control --summary-only
    cargo tarpaulin -p control --out Stdout
    .\scripts\control_coverage.ps1

## Module checklist (coverage_smoke + in-crate tests)

| Module | Covered by |
|--------|------------|
| api_key | in-crate + smoke |
| audit | in-crate + smoke |
| binding | in-crate |
| budget | in-crate |
| catalog | in-crate |
| dispatch | in-crate |
| health | in-crate + smoke |
| idempotency | in-crate + smoke |
| meter | in-crate + smoke |
| offer | in-crate |
| policy | in-crate + smoke |
| policy_templates | in-crate + smoke |
| principal | in-crate |
| provision | in-crate |
| rate_limit | in-crate + smoke |
| risk | in-crate |
| trace | in-crate |

