use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use entity::user::{ActiveModel, Entity, Model};
use sea_orm::{DbErr, EntityTrait, InsertResult, QuerySelect, RuntimeErr, Set};

pub async fn create_user(
    state: Arc<crate::AppState>,
    user_to_create: Model,
) -> Result<InsertResult<ActiveModel>, DbErr> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(user_to_create.password.as_bytes(), &salt)
        .unwrap();

    Entity::insert(ActiveModel {
        login: Set(user_to_create.login.to_owned()),
        password: Set(password_hash.to_string()),
    })
    .exec(&state.conn)
    .await
}

pub async fn check_password(
    state: Arc<crate::AppState>,
    user_to_check: Model,
) -> Result<bool, DbErr> {
    let user_pass = Entity::find_by_id(user_to_check.login.to_owned())
        .select_only()
        .one(&state.conn)
        .await?
        .map_or("".to_owned(), |u| u.password);

    let parsed_hash = PasswordHash::new(&user_pass).unwrap();
    if Argon2::default()
        .verify_password(user_to_check.password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        Ok(true)
    } else {
        Err(DbErr::Exec(RuntimeErr::Internal(
            "Password does not match".to_owned(),
        )))
    }
}
