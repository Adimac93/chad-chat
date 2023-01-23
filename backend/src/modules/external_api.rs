use anyhow::Error;
use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde::{de, Deserialize};
use serde_json::json;
use serde_json::Value;
use sqlx::query;
use sqlx::query_as;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::PgPool;
use tracing::debug;
use tracing_subscriber::field::debug;
use tracing_test::traced_test;

#[derive(Clone)]
pub struct HttpClient(Client);

impl HttpClient {
    pub fn new() -> Self {
        Self(Client::builder().user_agent("Chadnet").build().unwrap())
    }

    pub async fn parse_user_agent(&self, agent: &str) -> anyhow::Result<UserAgentParsed> {
        let res = self
            .0
            .post("https://user-agents.net/parser")
            .form(&json!({"string": agent, "action": "parse", "format": "json"}))
            .send()
            .await?;

        let user_agent = res.json::<UserAgentParsed>().await?;
        if user_agent.isfake {
            debug!("Aha! Fake user agent");
        }
        if user_agent.crawler {
            debug!("Aha! Crawler agent, beware of bots!");
        }

        Ok(user_agent)
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

        let geolocation = serde_json::from_value::<GeolocationParsed>(json)?;

        Ok(GeolocationData::new(geolocation))
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

impl GeolocationData {
    pub fn new(geo: GeolocationParsed) -> Self {
        Self {
            country: geo.country,
            region_name: geo.region_name,
            city: geo.city,
            zip: geo.zip,
            lat: geo.lat,
            lon: geo.lon,
            timezone: geo.timezone,
            isp: geo.isp,
            mobile: geo.mobile,
            proxy: geo.proxy,
            hosting: geo.hosting,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GeolocationParsed {
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

#[derive(Deserialize, Debug)]
pub struct UserAgentParsed {
    #[serde(deserialize_with = "bool_from_string")]
    activexcontrols: bool,
    #[serde(deserialize_with = "bool_from_string")]
    alpha: bool,
    aolversion: String,
    #[serde(deserialize_with = "bool_from_string")]
    backgroundsounds: bool,
    #[serde(deserialize_with = "bool_from_string")]
    beta: bool,
    pub browser: String,
    browser_bits: String,
    browser_maker: String,
    browser_modus: String,
    browser_type: String,
    comment: String,
    #[serde(deserialize_with = "bool_from_string")]
    cookies: bool,
    #[serde(deserialize_with = "bool_from_string")]
    pub crawler: bool, // is bot
    cssversion: String,
    pub device_brand_name: String,
    device_code_name: String,
    device_maker: String,
    pub device_name: String,
    device_pointing_method: String,
    pub device_type: String,
    #[serde(deserialize_with = "bool_from_string")]
    frames: bool,
    #[serde(deserialize_with = "bool_from_string")]
    iframes: bool,
    #[serde(deserialize_with = "bool_from_string")]
    isanonymized: bool,
    #[serde(deserialize_with = "bool_from_string")]
    pub isfake: bool,
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
    pub platform: String,
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

use crate::configuration::get_config;
use crate::modules::database::get_postgres_pool;

use super::extractors::user_agent::UserAgentData;
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
    let ip = IpAddr::from_str("194.79.23.20").unwrap();
    let geo = HttpClient::new().fetch_geolocation(ip).await.unwrap();
    debug!("{geo:#?}");
}

// #[traced_test]
// #[tokio::test]
// async fn fetch_ip_info_test_db() {
//     let cfg = get_config().unwrap();
//     let db = get_postgres_pool(cfg.postgres).await;
//     let ip = IpAddr::from_str("194.79.23.20").unwrap();
//     let geo = HttpClient::new().fetch_geolocation(ip).await.unwrap();
//     query!(
//         r#"
//             insert into user_networks (ip, is_trusted, geolocation_data)
//             values ($1, true, $2)
//         "#,
//         IpNetwork::from(ip),
//         sqlx::types::Json(&geo) as _
//     )
//     .execute(&db)
//     .await
//     .unwrap();
//     debug!("{geo:#?}");
// }

// #[tokio::test]
// async fn get_geo_db() {
//     let cfg = get_config().unwrap();
//     let db = get_postgres_pool(cfg.postgres).await;

//     let geo = query!(
//         r#"
//             select * from user_networks
//             join
//         "#
//     )
//     .fetch_one(&db)
//     .await
//     .unwrap()
//     .geolocation_data;

//     let res = serde_json::from_value::<GeolocationData>(geo).unwrap();
// }
