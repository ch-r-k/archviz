mod domain;
mod repository;
mod service;

use repository::PostgresUserRepository;
use service::UserService;

fn main() {
    let repo = PostgresUserRepository;

    let service = UserService {
        repo,
    };

    service.get_user();
}