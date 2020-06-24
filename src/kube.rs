use serde::{Deserialize, Serialize};
use std::fs;

pub fn new() -> anyhow::Result<CloudProviderConfig> {
    let contents: String = fs::read_to_string("/etc/kubernetes/azure.json")?;
    let result: CloudProviderConfig = serde_json::from_str::<CloudProviderConfig>(&contents)?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudProviderConfig {
    pub cloud: String,
    pub tenant_id: String,
    pub subscription_id: String,
    pub aad_client_id: String,
    pub aad_client_secret: String,
    pub resource_group: String,
    pub location: String,
    pub vm_type: String,
    pub subnet_name: String,
    pub security_group_name: String,
    pub vnet_name: String,
    pub vnet_resource_group: String,
    pub route_table_name: String,
    pub primary_availability_set_name: String,
    pub primary_scale_set_name: String,
    pub cloud_provider_backoff_mode: String,
    pub cloud_provider_backoff: bool,
    pub cloud_provider_backoff_retries: i64,
    pub cloud_provider_backoff_duration: i64,
    pub cloud_provider_ratelimit: bool,
    #[serde(rename = "cloudProviderRateLimitQPS")]
    pub cloud_provider_rate_limit_qps: i64,
    pub cloud_provider_rate_limit_bucket: i64,
    #[serde(rename = "cloudProviderRatelimitQPSWrite")]
    pub cloud_provider_ratelimit_qpswrite: i64,
    pub cloud_provider_ratelimit_bucket_write: i64,
    pub use_managed_identity_extension: bool,
    #[serde(rename = "userAssignedIdentityID")]
    pub user_assigned_identity_id: String,
    pub use_instance_metadata: bool,
    pub load_balancer_sku: String,
    #[serde(rename = "disableOutboundSNAT")]
    pub disable_outbound_snat: bool,
    #[serde(rename = "excludeMasterFromStandardLB")]
    pub exclude_master_from_standard_lb: bool,
    pub provider_vault_name: String,
    pub maximum_load_balancer_rule_count: i64,
    pub provider_key_name: String,
    pub provider_key_version: String,
}
