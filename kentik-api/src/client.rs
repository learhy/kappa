use futures_util::TryStreamExt;
use hyper::{Body, Client as HttpClient, Method, Request, Response, StatusCode};
use hyper::client::HttpConnector;
use hyper::http::request::Builder;
use hyper_rustls::HttpsConnector;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use rustls::ClientConfig;
use webpki_roots::TLS_SERVER_ROOTS;
use crate::Error;

#[derive(Clone)]
pub struct Client {
    pub(crate) client: HttpClient<HttpsConnector<HttpConnector>, Body>,
    pub(crate) email:  String,
    pub(crate) token:  String,
    pub(crate) urls:   Urls,
}

impl Client {
    pub fn new(email: &str, token: &str, region: Option<&str>) -> Client {
        let mut config = ClientConfig::new();
        config.root_store.add_server_trust_anchors(&TLS_SERVER_ROOTS);

        let mut c = HttpConnector::new();
        c.enforce_http(false);
        let c = HttpsConnector::from((c, config));

        Client {
            client: HttpClient::builder().build(c),
            email:  email.to_owned(),
            token:  token.to_owned(),
            urls:   Urls::new(region),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        let request  = self.request(url).body(Body::empty())?;
        let response = self.send(request).await?;
        let body     = response.into_body().try_concat().await?;
        Ok(serde_json::from_slice(&body)?)
    }

    pub async fn post<T: Serialize, U: DeserializeOwned>(&self, url: &str, body: &T) -> Result<U, Error> {
        let body = serde_json::to_vec(body)?;

        let mut request = self.request(url);
        request.method(Method::POST);
        request.header("Content-Type", "application/json");

        let request  = request.body(body.into())?;
        let response = self.send(request).await?;
        let body     = response.into_body().try_concat().await?;

        Ok(serde_json::from_slice(&body)?)
    }

    pub async fn send(&self, request: Request<Body>) -> Result<Response<Body>, Error> {
        let response = self.client.request(request).await?;
        let status   = response.status();
        match status {
            _ if status.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED => Err(Error::Auth),
            _                        => Err(error(response).await?),
        }
    }

    pub(crate) fn request(&self, url: &str) -> Builder {
        let mut builder = Builder::new();
        builder.uri(url);
        builder.header("X-CH-Auth-Email",     &self.email);
        builder.header("X-CH-Auth-API-Token", &self.token);
        builder
    }
}

async fn error(response: Response<Body>) -> Result<Error, Error> {
    let status = response.status();
    let body   = response.into_body().try_concat().await?;

    #[derive(Deserialize)]
    struct Wrapper {
        error: String,
    }

    Ok(match serde_json::from_slice::<Wrapper>(&body) {
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
