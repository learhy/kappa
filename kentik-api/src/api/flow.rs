use http::Method;
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use crate::{Client, Device, Error};

impl Client {
    pub async fn flow(&self, device: &Device, flow: Vec<u8>) -> Result<(), Error> {
        let cid = device.client_id();
        let url = format!("{}?sid=0&sender_id={}", self.urls.flow, cid);

        let mut request = self.request(Method::POST, &url)?;
        request.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/binary"));
        request.body_mut().replace(flow.into());
        self.send(request).await?;

        Ok(())
    }
}
