use serde::{Serialize, Deserialize};
use crate::{Client, Error};

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Device {
    #[serde(with = "crate::serde::str")]
    pub id: u64,
    #[serde(rename = "device_name")]
    pub name: String,
    #[serde(rename = "device_type")]
    pub kind: String,
    #[serde(rename = "device_subtype")]
    pub subtype: String,
    #[serde(rename = "device_bgp_type")]
    pub bgp_type: String,
    #[serde(rename = "cdn_attr")]
    pub cdn_attr: String,
    #[serde(rename = "device_sample_rate", with = "crate::serde::str")]
    pub sample_rate: u64,
    pub plan_id: Option<u64>,
    pub site_id: Option<u64>,
    #[serde(with = "crate::serde::str")]
    pub company_id: u64,
    #[serde(rename = "custom_column_data")]
    pub customs: Vec<Column>,
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    #[serde(rename = "field_id", with = "crate::serde::str")]
    pub id: u64,
    #[serde(rename = "col_name")]
    pub name: String,
    #[serde(rename = "col_type")]
    pub kind: String,
}

impl Device {
    pub fn client_id(&self) -> String {
        format!("{}:{}:{}", self.company_id, self.name, self.id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Wrapper {
    device: Device,
}

impl Client {
    pub async fn get_device_by_name(&self, name: &str) -> Result<Device, Error> {
        let url = format!("{}/device/{}", self.urls.internal, name);
        Ok(self.get::<Wrapper>(&url).await?.device)
    }

    pub async fn create_device(&self, device: Device) -> Result<Device, Error> {
        let url = format!("{}/device/", self.urls.internal);
        let arg = Wrapper { device };
        self.post::<Wrapper, Wrapper>(&url, &arg).await?;
        Ok(self.get_device_by_name(&arg.device.name).await?)
    }
}
