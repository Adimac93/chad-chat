use crate::modules::external_api::GeolocationData;
use crate::modules::extractors::geolocation::NetworkData;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::types::Json;
use sqlx::{query, query_as, Acquire, PgConnection, Postgres};
use uuid::Uuid;

#[derive(Debug)]
pub struct UserNetworkData {
    ip: IpNetwork,
    geo: GeolocationData,
    is_trusted: bool,
}

pub struct NetworkQuery<'c> {
    conn: &'c mut PgConnection,
    ip: IpNetwork,
}

impl<'c> NetworkQuery<'c> {
    pub fn new(ip: IpNetwork, conn: &'c mut PgConnection) -> Self {
        Self { conn, ip }
    }

    pub async fn add_network(&mut self, geo: GeolocationData) -> anyhow::Result<()> {
        query!(
            r#"
                insert into networks (ip, geolocation_data)
                values ($1, $2)
            "#,
            self.ip,
            Json(geo) as _
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn assign(&mut self, user_id: &Uuid, is_trusted: bool) -> anyhow::Result<()> {
        query!(
            r#"
                insert into user_networks (network_ip, user_id, is_trusted)
                values ($1, $2, $3)
            "#,
            self.ip,
            user_id,
            is_trusted
        )
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    pub async fn get_all(&mut self) -> anyhow::Result<Vec<NetworkData>> {
        let res = query_as!(
            NetworkData,
            r#"
                select ip as "ip: IpNetwork", geolocation_data as "geolocation_data: GeolocationData" from networks
            "#
        ).fetch_all(&mut *self.conn).await?;
        Ok(res)
    }

    pub async fn get_all_user(&mut self, user_id: &Uuid) -> anyhow::Result<Vec<UserNetworkData>> {
        let res = query_as!(
            UserNetworkData,
            r#"
                select n.ip as "ip: IpNetwork", n.geolocation_data as "geo: GeolocationData", un.is_trusted from user_networks un
                join networks n on n.ip = un.network_ip
                where user_id = $1
            "#,
            user_id
        ).fetch_all(&mut *self.conn).await?;
        Ok(res)
    }

    pub async fn is_new(&mut self) -> anyhow::Result<bool> {
        let res = query!(
            r#"
                select * from networks
                where ip = $1
            "#,
            self.ip
        )
        .fetch_optional(&mut *self.conn)
        .await?
        .is_none();

        Ok(res)
    }

    pub async fn is_trusted(&mut self, user_id: &Uuid) -> anyhow::Result<bool> {
        let res = query!(
            r#"
                select is_trusted from user_networks
                where user_id = $1 and network_ip = $2
            "#,
            user_id,
            &self.ip
        )
        .fetch_one(&mut *self.conn)
        .await?
        .is_trusted;
        Ok(res)
    }
}
