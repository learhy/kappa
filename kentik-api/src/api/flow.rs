use crate::{Client, Device, Error};

impl Client {
    pub async fn flow(&self, device: &Device, flow: Vec<u8>) -> Result<(), Error> {
        let cid = device.client_id();
        let url = format!("{}?sid=0&sender_id={}", self.urls.flow, cid);

        let mut request = self.request(&url);
        request.header("Content-Type", "application/binary");
        let request = request.body(flow.into())?;
        self.send(request).await?;

        Ok(())
    }
}
