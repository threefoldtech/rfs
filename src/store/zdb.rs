use super::{Error, Result, Route, Store};
use anyhow::Context;

use bb8_redis::{
    bb8::{CustomizeConnection, Pool},
    redis::{
        aio::Connection, cmd, AsyncCommands, ConnectionAddr, ConnectionInfo, RedisConnectionInfo,
        RedisError,
    },
    RedisConnectionManager,
};

#[derive(Debug)]
struct WithNamespace {
    namespace: Option<String>,
    password: Option<String>,
}

#[async_trait::async_trait]
impl CustomizeConnection<Connection, RedisError> for WithNamespace {
    async fn on_acquire(&self, connection: &mut Connection) -> anyhow::Result<(), RedisError> {
        match self.namespace {
            Some(ref ns) if ns != "default" => {
                let mut c = cmd("SELECT");
                let c = c.arg(ns);
                if let Some(ref password) = self.password {
                    c.arg(password);
                }

                let result = c.query_async(connection).await;
                if let Err(ref err) = result {
                    error!("failed to switch namespace to {}: {}", ns, err);
                }
                result
            }
            _ => Ok(()),
        }
    }
}

pub struct ZdbStoreFactory;

fn get_connection_info<U: AsRef<str>>(u: U) -> Result<(ConnectionInfo, Option<String>)> {
    let u = url::Url::parse(u.as_ref())?;

    let (address, namespace) = match u.host() {
        Some(host) => {
            let addr = match host {
                url::Host::Domain(domain) => domain.to_owned(),
                url::Host::Ipv4(ipv4) => ipv4.to_string(),
                url::Host::Ipv6(ipv6) => ipv6.to_string(),
            };

            let addr = ConnectionAddr::Tcp(addr, u.port().unwrap_or(9900));
            let ns: Option<String> = u
                .path_segments()
                .and_then(|s| s.last().map(|s| s.to_owned()));
            (addr, ns)
        }
        None => (ConnectionAddr::Unix(u.path().into()), None),
    };

    Ok((
        ConnectionInfo {
            addr: address,
            redis: RedisConnectionInfo {
                db: 0,
                username: if u.username().is_empty() {
                    None
                } else {
                    Some(u.username().into())
                },
                password: u.password().map(|s| s.into()),
            },
        },
        namespace,
    ))
}

#[derive(Clone)]
pub struct ZdbStore {
    url: String,
    pool: Pool<RedisConnectionManager>,
}

impl ZdbStore {
    pub async fn make<U: AsRef<str>>(url: &U) -> Result<ZdbStore> {
        let (mut info, namespace) = get_connection_info(url.as_ref())?;

        let namespace = WithNamespace {
            namespace,
            password: info.redis.password.take(),
        };

        log::debug!("connection {:#?}", info);
        log::debug!("switching namespace to: {:?}", namespace.namespace);

        let mgr = RedisConnectionManager::new(info)
            .context("failed to create redis connection manager")?;

        let pool = Pool::builder()
            .max_size(20)
            .connection_customizer(Box::new(namespace))
            .build(mgr)
            .await
            .context("failed to create connection pool")?;

        Ok(ZdbStore {
            url: url.as_ref().to_string(),
            pool,
        })
    }
}

#[async_trait::async_trait]
impl Store for ZdbStore {
    async fn get(&self, key: &[u8]) -> super::Result<Vec<u8>> {
        let mut con = self.pool.get().await.context("failed to get connection")?;

        let result: Option<Vec<u8>> = con.get(key).await.context("failed to get blob")?;
        let result = result.ok_or(Error::KeyNotFound)?;

        if result.is_empty() {
            return Err(Error::InvalidBlob);
        }

        Ok(result)
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        let mut con = self.pool.get().await.context("failed to get connection")?;

        if con
            .exists(key)
            .await
            .context("failed to check if blob exists")?
        {
            return Ok(());
        };

        con.set(key, blob).await.context("failed to set blob")?;

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        vec![Route::url(self.url.clone())]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_connection_info_simple() {
        let (info, ns) = get_connection_info("zdb://hub.grid.tf:9900").unwrap();
        assert_eq!(ns, None);
        assert_eq!(info.addr, ConnectionAddr::Tcp("hub.grid.tf".into(), 9900));
    }

    #[test]
    fn test_connection_info_ns() {
        let (info, ns) = get_connection_info("zdb://username@hub.grid.tf/custom").unwrap();
        assert_eq!(ns, Some("custom".into()));
        assert_eq!(info.addr, ConnectionAddr::Tcp("hub.grid.tf".into(), 9900));
        assert_eq!(info.redis.username, Some("username".into()));
    }

    #[test]
    fn test_connection_info_unix() {
        let (info, ns) = get_connection_info("zdb:///path/to/socket").unwrap();
        assert_eq!(ns, None);
        assert_eq!(info.addr, ConnectionAddr::Unix("/path/to/socket".into()));
    }
}
