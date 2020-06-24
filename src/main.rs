use anyhow::Result;

mod imds;
mod kube;
mod oauth;
mod skus;

fn main() -> Result<()> {
    smol::run(async {
        println!("Hello, world!");
        let azure_json = kube::new()?;

        println!("{:#?}", &azure_json);

        let _meta = imds::new().await?;

        let client_id: &str;
        if &azure_json.aad_client_id == "msi" {
            client_id = &azure_json.user_assigned_identity_id;
        } else {
            client_id = &azure_json.aad_client_id
        }

        let token = oauth::new(&client_id).await?;
        let skus: Vec<skus::Resource> = skus::new(&token.access_token, &azure_json.subscription_id)
            .await?
            .value;

        println!("{:#?}", &azure_json.location);

        let skus: Vec<skus::Resource> = skus
            .into_iter()
            .filter(|sku| sku.locations[0] == azure_json.location)
            .filter(|sku| sku.resource_type == "disks")
            .filter(|sku| sku.size.is_some())
            .filter(|sku| sku.size.clone().unwrap().contains("P80"))
            .collect();

        println!("{:#?}", &skus[..]);

        Ok(())
    })
}
