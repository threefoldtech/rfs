use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

pub trait DB: Send + Sync {
    fn get_user_by_username(&self, username: &str) -> Option<&User>;
}

#[derive(Debug, ToSchema)]
pub struct VecDB {
    users: Vec<User>,
}

impl VecDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: users.to_vec(),
        }
    }
}

impl DB for VecDB {
    fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.users.iter().find(|u| u.username == username)
    }
}
