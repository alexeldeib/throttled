use serde::{Deserialize, Serialize};

pub async fn new(token: &str, subscription_id: &str) -> anyhow::Result<SKUResponse> {
    let res = reqwest::Client::new()
        .get(&format!(
            "https://management.azure.com/subscriptions/{}/providers/Microsoft.Compute/skus",
            subscription_id
        ))
        .header("Authorization", &format!("Bearer {}", token))
        .query(&[("api-version", "2019-04-01")])
        .send()
        .await?
        .text()
        .await?;

    println!("{:#?}", &res[0..1000]);

    let res: SKUResponse = serde_json::from_str(&res[..])?;

    Ok(res)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SKUResponse {
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
