use std::io::prelude::*;
use flate2::{Compression, write::GzEncoder};
use http::Method;
use reqwest::header::{CONTENT_TYPE, CONTENT_ENCODING, HeaderValue};
use crate::{Client, Device, Error};

impl Client {
    pub async fn flow(&self, device: &Device, flow: Vec<u8>) -> Result<(), Error> {
        let cid = device.client_id();
        let url = format!("{}?sid=0&sender_id={}", self.urls.flow, cid);

        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(&flow)?;
        let flow = e.finish()?;

        let mut request = self.request(Method::POST, &url)?;
        request.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/binary"));
        request.headers_mut().insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        request.body_mut().replace(flow.into());
        self.send(request).await?;

        Ok(())
    }
}
