use std::collections::HashMap;
use utoipa::ToSchema;

use super::DB;
use crate::models::User;

#[derive(Debug, ToSchema)]
pub struct MapDB {
    users: HashMap<String, User>,
}

impl MapDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: users
                .iter()
                .map(|u| (u.username.clone(), u.to_owned()))
                .collect(),
        }
    }
}

impl DB for MapDB {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users.get(username).cloned()
    }
}
