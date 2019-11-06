use futures_util::TryStreamExt;
use crate::{Client, Device, Error};

impl Client {
    pub async fn flow(&self, device: &Device, flow: Vec<u8>) -> Result<(), Error> {
        let cid = device.client_id();
        let url = format!("{}?sid=0&sender_id={}", self.urls.flow, cid);

        let mut request = self.request(&url);
        request.header("Content-Type", "application/binary");

        let request  = request.body(flow.into())?;
        let response = self.send(request).await?;
        response.into_body().try_concat().await?;

        Ok(())
    }
}
