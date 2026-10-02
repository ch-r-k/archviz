use crate::repository::UserRepository;

pub struct UserService<R>
where
    R: UserRepository,
{
    pub repo: R,
}

impl<R> UserService<R>
where
    R: UserRepository,
{
    pub fn get_user(&self) {
        let user = self.repo.find_user();

        println!("{:?}", user);
    }
}