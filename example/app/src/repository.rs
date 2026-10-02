use crate::domain::User;

pub trait UserRepository {
    fn find_user(&self) -> User;
}

pub struct PostgresUserRepository;

impl UserRepository for PostgresUserRepository {
    fn find_user(&self) -> User {
        User {
            id: 1,
            name: "Alice".into(),
        }
    }
}