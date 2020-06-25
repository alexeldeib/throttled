use eyre::{eyre, Context, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

pub async fn list_skus(token: &str, subscription_id: &str) -> Result<ResourceList> {
    let res = reqwest::Client::new()
        .get(&format!(
            "https://management.azure.com/subscriptions/{}/providers/Microsoft.Compute/skus",
            subscription_id
        ))
        .header("Authorization", &format!("Bearer {}", token))
        .query(&[("api-version", "2019-04-01")])
        .send()
        .await
        .wrap_err_with(|| "failed to fetch resource skus")?
        .text()
        .await
        .wrap_err_with(|| "failed to receive resource skus response")?;

    let res: ResourceList = serde_json::from_str(&res[..])
        .wrap_err_with(|| "failed to parse resource skus from response")?;

    Ok(res)
}

pub fn with_name(name: String) -> impl Fn(&Resource) -> bool {
    move |res: &Resource| res.name == name
}

pub fn with_location(location: String) -> impl Fn(&Resource) -> bool {
    move |res: &Resource| res.locations.len() > 0 && res.locations[0] == location
}

pub fn with_resource_type(resource_type: String) -> impl Fn(&Resource) -> bool {
    move |res: &Resource| res.resource_type == resource_type
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceList {
    pub value: Vec<Resource>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    pub family: Option<String>,
    pub location_info: Vec<LocationInfo>,
    pub locations: Vec<String>,
    pub name: String,
    pub resource_type: String,
    pub size: Option<String>,
    pub tier: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub name: String,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    pub location: String,
    pub zone_details: Vec<ZoneDetail>,
    pub zones: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneDetail {
    #[serde(rename = "Name")]
    pub name: Vec<String>,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Default)]
pub struct VirtualMachine {
    pub name: String,
    pub location: String,
    pub max_iops: u64,
    pub max_bandwidth: u64,
}

impl TryFrom<Resource> for VirtualMachine {
    type Error = Error;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        if value.locations.len() < 1 {
            return Err(eyre!("no locations available for provided sku"));
        }

        let name = value.name.clone();
        let location = value.locations[0].clone();
        let mut capabilities: HashMap<String, String> = HashMap::new();
        for cap in value.capabilities.clone() {
            capabilities.entry(cap.name).or_insert(cap.value);
        }

        let max_iops = capabilities
            .get("UncachedDiskIOPS")
            .ok_or(eyre!("failed to find minimum vm sku size: {:#?}", &value))?
            .parse::<u64>()?;

        // TODO(ace): for some reason SKUs API doesn't return bandwidth
        // numbers for b12ms. Everything above b8ms has the same limit,
        // so we can work around it. Probably ask CRP why this is
        // unpopulated. Because we accept the sku name filter before
        // invoking try_from, this only affects users on bad skus like b12ms.
        let max_bandwidth = capabilities
            .get("UncachedDiskBytesPerSecond")
            .ok_or(eyre!("failed to find maximum vm sku bandwidth"))?
            .parse::<u64>()?;

        let sku = VirtualMachine {
            name,
            location,
            max_iops,
            max_bandwidth,
        };

        Ok(sku)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Disk {
    pub location: String,
    pub storage_account_type: String,
    pub min_size_gb: u64,
    pub max_size_gb: u64,
    pub max_iops: u64,
    pub max_bandwidth: u64,
}

impl TryFrom<Resource> for Disk {
    type Error = Error;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        if value.locations.len() < 1 {
            return Err(eyre!("no locations available for provided sku"));
        }

        let location = value.locations[0].clone();
        let storage_account_type = value.name.clone();
        let mut capabilities: HashMap<String, String> = HashMap::new();
        for cap in value.capabilities.clone() {
            capabilities.entry(cap.name).or_insert(cap.value);
        }

        // Azure provides VM bandwith limits in B/s but disk limits in MB/s
        let base: u64 = 2;
        let exp = |n| -> u64 { n * base.pow(20) };

        let min_size_gb = capabilities
            .get("MinSizeGiB")
            .ok_or(eyre!("failed to find minimum disk sku size"))?
            .parse::<u64>()?;

        let max_size_gb = capabilities
            .get("MaxSizeGiB")
            .ok_or(eyre!("failed to find maximum disk sku size"))?
            .parse::<u64>()?;

        let max_iops = capabilities
            .get("MaxIOps")
            .ok_or(eyre!("failed to find maximum disk sku iops"))?
            .parse::<u64>()?;

        let max_bandwidth = capabilities
            .get("MaxBandwidthMBps")
            .ok_or(eyre!("failed to find maximum disk sku bandwidth"))?
            .parse::<u64>()
            .map(exp)?;

        let sku = Disk {
            location,
            storage_account_type,
            min_size_gb,
            max_size_gb,
            max_iops,
            max_bandwidth,
        };

        Ok(sku)
    }
}

pub fn with_disk_size(size: u64) -> impl Fn(&Disk) -> bool {
    move |sku: &Disk| size <= sku.max_size_gb && size > sku.min_size_gb
}
