use serde::{Deserialize, Serialize};

pub async fn new(client_id: &str) -> anyhow::Result<OAuthResponse> {
    let res = reqwest::Client::new()
        .get("http://169.254.169.254/metadata/identity/oauth2/token")
        .header("Metadata", "true")
        .query(&[
            ("client_id", client_id),
            ("api-version", "2018-02-01"),
            ("resource", "https://management.azure.com/"),
        ])
        .send()
        .await?
        .json::<OAuthResponse>()
        .await?;

    Ok(res)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OAuthResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: String,
    pub expires_on: String,
    pub not_before: String,
    pub resource: String,
    pub token_type: String,
}
