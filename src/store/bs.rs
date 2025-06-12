use super::{Error, Result, Store};
use crate::fungi::meta::Block;
use aes_gcm::{
    aead::{
        generic_array::{self, GenericArray},
        Aead, KeyInit,
    },
    Aes256Gcm, Nonce,
};

fn hash(input: &[u8]) -> GenericArray<u8, generic_array::typenum::U32> {
    let hash = blake2b_simd::Params::new().hash_length(32).hash(input);
    GenericArray::from_slice(hash.as_bytes()).to_owned()
}

/// The block store builds on top of a store and adds encryption and compression
#[derive(Clone, Debug)]
pub struct BlockStore<S: Store> {
    store: S,
}

impl<S> From<S> for BlockStore<S>
where
    S: Store,
{
    fn from(store: S) -> Self {
        Self { store }
    }
}

impl<S> BlockStore<S>
where
    S: Store,
{
    pub fn inner(self) -> S {
        self.store
    }

    pub async fn get(&self, block: &Block) -> Result<Vec<u8>> {
        let encrypted = self.store.get(&block.id).await?;

        let cipher = Aes256Gcm::new_from_slice(&block.key).map_err(|_| Error::InvalidKey)?;
        let nonce = Nonce::from_slice(&block.key[..12]);

        let compressed = cipher
            .decrypt(nonce, encrypted.as_slice())
            .map_err(|_| Error::EncryptionError)?;

        let mut decoder = snap::raw::Decoder::new();
        let plain = decoder.decompress_vec(&compressed)?;

        Ok(plain)
    }

    pub async fn set(&self, blob: &[u8]) -> Result<Block> {
        // we first calculate the hash of the plain-text data

        let key = hash(blob);
        let mut encoder = snap::raw::Encoder::new();
        // data is then compressed
        let compressed = encoder.compress_vec(blob)?;

        // we then encrypt it using the hash of the plain-text as a key
        let cipher = Aes256Gcm::new(&key);
        // the nonce is still driven from the key, a nonce is 12 bytes for aes
        // it's done like this so a store can still dedup the data
        let nonce = Nonce::from_slice(&key[..12]);

        // we encrypt the data
        let encrypted = cipher
            .encrypt(nonce, compressed.as_slice())
            .map_err(|_| Error::EncryptionError)?;

        // we hash it again, and use that as the store key
        let id = hash(&encrypted);

        let block = Block {
            id: id.into(),
            key: key.into(),
        };

        self.store.set(&block.id, &encrypted).await?;

        Ok(block)
    }
}

#[cfg(test)]
mod test {
    use super::super::Route;

    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct InMemoryStore {
        map: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    }

    #[async_trait::async_trait]
    impl Store for InMemoryStore {
        async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
            let map = self.map.lock().await;
            let v = map.get(key).ok_or(Error::KeyNotFound)?;
            Ok(v.clone())
        }
        async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
            let mut map = self.map.lock().await;
            map.insert(key.into(), blob.into());

            Ok(())
        }

        fn routes(&self) -> Vec<Route> {
            vec![Route::url("mem://")]
        }
    }

    #[tokio::test]
    async fn test_block_store() {
        let store = InMemoryStore::default();
        let block_store = BlockStore::from(store);

        let blob = "some random data to store";
        let block = block_store.set(blob.as_bytes()).await.unwrap();

        let received = block_store.get(&block).await.unwrap();

        assert_eq!(blob.as_bytes(), received.as_slice());
    }
}
