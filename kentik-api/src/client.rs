use http::{Method, StatusCode};
use reqwest::{Client as HttpClient, Request, Response};
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::Error;

pub struct Client {
    pub(crate) client: HttpClient,
    pub(crate) email:  HeaderValue,
    pub(crate) token:  HeaderValue,
    pub(crate) urls:   Urls,
}

impl Client {
    pub fn new(email: &str, token: &str, region: Option<&str>) -> Result<Self, Error> {
        Ok(Self {
            client: HttpClient::new(),
            email:  HeaderValue::from_str(email)?,
            token:  HeaderValue::from_str(token)?,
            urls:   Urls::new(region),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        let request  = self.request(Method::GET, url)?;
        let response = self.send(request).await?;
        Ok(response.json().await?)
    }

    pub async fn post<T: Serialize, U: DeserializeOwned>(&self, url: &str, body: &T) -> Result<U, Error> {
        let body = serde_json::to_vec(body)?;

        let mut request = self.request(Method::POST, url)?;
        request.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        request.body_mut().replace(body.into());

        let response = self.send(request).await?;
        Ok(response.json().await?)
    }

    pub async fn send(&self, request: Request) -> Result<Response, Error> {
        let response = self.client.execute(request).await?;
        let status   = response.status();
        match status {
            _ if status.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED => Err(Error::Auth),
            _                        => Err(error(response).await?),
        }
    }

    pub(crate) fn request(&self, method: Method, url: &str) -> Result<Request, Error> {
        let mut req = Request::new(method, url.parse()?);
        req.headers_mut().insert("X-CH-Auth-Email",     self.email.clone());
        req.headers_mut().insert("X-CH-Auth-API-Token", self.token.clone());
        Ok(req)
    }
}

async fn error(response: Response) -> Result<Error, Error> {
    let status = response.status();

    #[derive(Deserialize)]
    struct Wrapper {
        error: String,
    }

    Ok(match response.json::<Wrapper>().await {
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
