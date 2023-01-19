use anyhow::Error;
use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde::{de, Deserialize};
use serde_json::json;
use serde_json::Value;
use tracing::debug;
use tracing_subscriber::field::debug;
use tracing_test::traced_test;

#[derive(Clone)]
pub struct HttpClient(Client);

impl HttpClient {
    pub fn new() -> Self {
        Self(Client::builder().user_agent("Chadnet").build().unwrap())
    }

    pub async fn parse_user_agent(&self, agent: &str) -> anyhow::Result<UserAgentData> {
        let res = self
            .0
            .post("https://user-agents.net/parser")
            .form(&json!({"string": agent, "action": "parse", "format": "json"}))
            .send()
            .await?;

        let user_agent = res.json::<ParsedUserAgent>().await?;
        if user_agent.isfake {
            debug!("Aha! Fake user agent");
        }
        if user_agent.crawler {
            debug!("Aha! Crawler agent, beware of bots!");
        }

        Ok(UserAgentData::new(user_agent))
    }

    pub async fn fetch_geolocation(&self, ip: IpAddr) -> anyhow::Result<GeolocationData> {
        let fields = 18596857; // https://ip-api.com/docs/api:json#test
        let url = format!("http://ip-api.com/json/{ip}?fields={fields}");
        debug!("Geolocation ip: {ip}");
        let res = self.0.get(url).send().await?;

        let json = res.json::<Value>().await?;
        if json["status"] == "fail" {
            return Err(anyhow::Error::msg(json["message"].clone()));
        }

        let geolocation = serde_json::from_value::<GeolocationData>(json)?;

        Ok(geolocation)
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "geolocation_data")]
pub struct GeolocationData {
    country: String,
    #[serde(rename(deserialize = "regionName"))]
    region_name: String,
    city: String,
    zip: String,
    lat: f32,
    lon: f32,
    timezone: String,
    isp: String,
    mobile: bool,
    proxy: bool,
    hosting: bool,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "user_agent_data")]
pub struct UserAgentData {
    browser: String,
    device_brand_name: String,
    device_name: String,
    device_type: String,
    platform: String,
    crawler: bool,
    is_fake: bool,
}

impl UserAgentData {
    fn new(ua: ParsedUserAgent) -> Self {
        Self {
            browser: ua.browser,
            device_brand_name: ua.device_brand_name,
            device_name: ua.device_name,
            device_type: ua.device_type,
            platform: ua.platform,
            crawler: ua.crawler,
            is_fake: ua.isfake,
        }
    }
    pub fn is_trusted(&self) -> bool {
        !(self.crawler || self.is_fake)
    }
}

#[derive(Deserialize, Debug)]
pub struct ParsedUserAgent {
    #[serde(deserialize_with = "bool_from_string")]
    activexcontrols: bool,
    #[serde(deserialize_with = "bool_from_string")]
    alpha: bool,
    aolversion: String,
    #[serde(deserialize_with = "bool_from_string")]
    backgroundsounds: bool,
    #[serde(deserialize_with = "bool_from_string")]
    beta: bool,
    browser: String,
    browser_bits: String,
    browser_maker: String,
    browser_modus: String,
    browser_type: String,
    comment: String,
    #[serde(deserialize_with = "bool_from_string")]
    cookies: bool,
    #[serde(deserialize_with = "bool_from_string")]
    crawler: bool, // is bot
    cssversion: String,
    device_brand_name: String,
    device_code_name: String,
    device_maker: String,
    device_name: String,
    device_pointing_method: String,
    device_type: String,
    #[serde(deserialize_with = "bool_from_string")]
    frames: bool,
    #[serde(deserialize_with = "bool_from_string")]
    iframes: bool,
    #[serde(deserialize_with = "bool_from_string")]
    isanonymized: bool,
    #[serde(deserialize_with = "bool_from_string")]
    isfake: bool,
    #[serde(deserialize_with = "bool_from_string")]
    ismobiledevice: bool,
    #[serde(deserialize_with = "bool_from_string")]
    ismodified: bool,
    #[serde(deserialize_with = "bool_from_string")]
    issyndicationreader: bool,
    #[serde(deserialize_with = "bool_from_string")]
    istablet: bool,
    #[serde(deserialize_with = "bool_from_string")]
    javaapplets: bool,
    #[serde(deserialize_with = "bool_from_string")]
    javascript: bool,
    majorver: String,
    minorver: String,
    parent: String,
    platform: String,
    platform_bits: String,
    platform_description: String,
    platform_maker: String,
    platform_version: String,
    renderingengine_description: String,
    renderingengine_maker: String,
    renderingengine_name: String,
    renderingengine_version: String,
    #[serde(deserialize_with = "bool_from_string")]
    tables: bool,
    #[serde(deserialize_with = "bool_from_string")]
    vbscript: bool,
    version: String,
    #[serde(deserialize_with = "bool_from_string")]
    win16: bool,
    #[serde(deserialize_with = "bool_from_string")]
    win32: bool,
    #[serde(deserialize_with = "bool_from_string")]
    win64: bool,
}

use std::net::IpAddr;
use std::str::FromStr;
fn bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let val: &str = de::Deserialize::deserialize(deserializer)?;
    let res =
        bool::from_str(val).map_err(|_| de::Error::unknown_variant(val, &["true", "false"]))?;
    Ok(res)
}

#[traced_test]
#[tokio::test]
async fn parse_user_agent_test() {
    let agent =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:108.0) Gecko/20100101 Firefox/108.0";

    let user_agent_data = HttpClient::new().parse_user_agent(agent).await.unwrap();
    debug!("{user_agent_data:#?}");
}

#[traced_test]
#[tokio::test]
async fn fetch_ip_info_test() {
    let ip = IpAddr::from_str("194.79.23.4").unwrap();
    let geo = HttpClient::new().fetch_geolocation(ip).await.unwrap();
    debug!("{geo:#?}");
}
