use crate::cache::{ConnectionInfo, IntoConnectionInfo};
use bb8_redis::redis::{
    ConnectionAddr, ConnectionInfo as RedisConnectionInfo,
    RedisConnectionInfo as InnerConnectionInfo,
};
use serde::{Deserialize, Serialize};

/// Backend is the backend (storage) information
/// stored in the metadata (flist)
#[derive(Debug, Serialize, Deserialize)]
pub struct Backend {
    host: String,
    port: u16,
    socket: Option<String>,
    namespace: Option<String>,
    password: Option<String>,
}

impl Backend {
    pub fn load(data: &[u8]) -> Result<Backend, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

impl IntoConnectionInfo for Backend {
    fn into_connection_info(self) -> anyhow::Result<ConnectionInfo> {
        let redis = RedisConnectionInfo {
            addr: ConnectionAddr::Tcp(self.host, self.port),
            redis: InnerConnectionInfo::default(),
        };

        Ok(ConnectionInfo::new(redis, self.namespace))
    }
}
