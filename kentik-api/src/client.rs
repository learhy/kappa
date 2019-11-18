use isahc::prelude::*;
use http::{Method, StatusCode, request::Builder};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::Error;

pub struct Client {
    pub(crate) client: HttpClient,
    pub(crate) email:  String,
    pub(crate) token:  String,
    pub(crate) urls:   Urls,
}

impl Client {
    pub fn new(email: &str, token: &str, region: Option<&str>) -> Result<Self, Error> {
        Ok(Self {
            client: HttpClient::new()?,
            email:  email.to_owned(),
            token:  token.to_owned(),
            urls:   Urls::new(region),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        let request = self.request(url).body(Body::empty())?;
        let mut res = self.send(request).await?;
        Ok(res.body_mut().json()?)
    }

    pub async fn post<T: Serialize, U: DeserializeOwned>(&self, url: &str, body: &T) -> Result<U, Error> {
        let body = serde_json::to_vec(body)?;

        let mut request = self.request(url);
        request.method(Method::POST);
        request.header("Content-Type", "application/json");

        let request = request.body(body.into())?;
        let mut res = self.send(request).await?;

        Ok(res.body_mut().json()?)
    }

    pub async fn send(&self, request: Request<Body>) -> Result<Response<Body>, Error> {
        let response = self.client.send_async(request).await?;
        let status   = response.status();
        match status {
            _ if status.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED => Err(Error::Auth),
            _                        => Err(error(response).await?),
        }
    }

    pub(crate) fn request(&self, url: &str) -> Builder {
        let mut builder = Request::builder();
        builder.uri(url);
        builder.header("X-CH-Auth-Email",     &self.email);
        builder.header("X-CH-Auth-API-Token", &self.token);
        builder
    }
}

async fn error(mut response: Response<Body>) -> Result<Error, Error> {
    let status = response.status();
    let body   = response.body_mut();

    #[derive(Deserialize)]
    struct Wrapper {
        error: String,
    }

    Ok(match body.json::<Wrapper>() {
        Ok(w)  => Error::App(w.error, status.into()),
        Err(_) => Error::Status(status.into()),
    })
}

#[derive(Clone)]
pub struct Urls {
    pub api:      String,
    pub dns:      String,
    pub flow:     String,
    pub internal: String,
}

impl Urls {
    fn new(region: Option<&str>) -> Self {
        let region = region.unwrap_or("US");

        let domain = match region.to_ascii_uppercase().as_ref() {
            "US" => "kentik.com".to_owned(),
            "EU" => "kentik.eu".to_owned(),
            name => format!("{}.kentik.com", name.to_ascii_lowercase()),
        };

        if region.starts_with("localhost") {
            return Self {
                api:      format!("http://{}/api/v5",       region),
                dns:      format!("http://{}/dns",          region),
                flow:     format!("http://{}/chf",          region),
                internal: format!("http://{}/api/internal", region),
            }
        }

        Self {
            api:      format!("https://api.{}/api/v5",       domain),
            dns:      format!("https://flow.{}/dns",         domain),
            flow:     format!("https://flow.{}/chf",         domain),
            internal: format!("https://api.{}/api/internal", domain),
        }
    }
}
