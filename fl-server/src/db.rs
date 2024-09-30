use std::{collections::HashMap, sync::Mutex};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

pub trait DB: Send + Sync {
    fn get_user_by_username(&self, username: &str) -> Option<User>;
}

#[derive(Debug, ToSchema)]
pub struct MapDB {
    users: Mutex<HashMap<String, User>>,
}

impl MapDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: Mutex::new(
                users
                    .to_vec()
                    .iter()
                    .map(|u| (u.username.clone(), u.to_owned()))
                    .collect(),
            ),
        }
    }
}

impl DB for MapDB {
    fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users
            .lock()
            .expect("failed to lock users map")
            .get(username)
            .cloned()
    }
}
