use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::prelude::*;

use super::async_pool;
use super::errors::DieselRepoError;
use super::infra;
use super::schema::users;

use crate::core::{QueryParams, RepoResult, ResultPaging};
use crate::users::{User, UserRepo, UserUpdate};

#[derive(Queryable, Insertable)]
#[table_name = "users"]
struct UserDiesel {
    id: String,
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    created_by: String,
    updated_by: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Into<User> for UserDiesel {
    fn into(self) -> User {
        User {
            id: self.id,
            first_name: self.first_name,
            last_name: self.last_name,
            email: self.email,
            password: self.password,
            created_at: self.created_at,
            created_by: self.created_by,
            updated_at: self.updated_at,
            updated_by: self.updated_by,
        }
    }
}

impl From<User> for UserDiesel {
    fn from(u: User) -> Self {
        UserDiesel {
            id: u.id,
            first_name: u.first_name,
            last_name: u.last_name,
            email: u.email,
            password: u.password,
            created_at: u.created_at,
            created_by: u.created_by,
            updated_at: u.updated_at,
            updated_by: u.updated_by,
        }
    }
}

#[derive(Debug, Clone, AsChangeset)]
#[table_name = "users"]
struct UserUpdateDiesel {
    first_name: String,
    last_name: String,
    email: String,
    updated_by: String,
    updated_at: NaiveDateTime,
}

impl From<UserUpdate> for UserUpdateDiesel {
    fn from(u: UserUpdate) -> Self {
        UserUpdateDiesel {
            first_name: u.first_name,
            last_name: u.last_name,
            email: u.email,
            updated_at: u.updated_at,
            updated_by: u.updated_by,
        }
    }
}

pub struct UserDieselImpl {
    pool: Arc<infra::DBConn>,
}

impl UserDieselImpl {
    pub fn new(db: Arc<infra::DBConn>) -> Self {
        UserDieselImpl { pool: db }
    }

    async fn total(&self) -> RepoResult<i64> {
        use super::schema::users::dsl::users;
        let pool = self.pool.clone();
        async_pool::run(move || {
            let mut conn = pool.get().unwrap();
            users.count().get_result(&mut conn)
        })
        .await
        .map_err(|v| DieselRepoError::from(v).into_inner())
        .map(|v: i64| v)
    }

    async fn fetch(&self, query: &dyn QueryParams) -> RepoResult<Vec<User>> {
        use super::schema::users::dsl::users;
        let pool = self.pool.clone();
        let builder = users.limit(query.limit()).offset(query.offset());
        let result = async_pool::run(move || {
            let mut conn = pool.get().unwrap();
            builder.load::<UserDiesel>(&mut conn)
        })
        .await
        .map_err(|v| DieselRepoError::from(v).into_inner())?;
        Ok(result.into_iter().map(|v| -> User { v.into() }).collect())
    }
}

#[async_trait]
impl UserRepo for UserDieselImpl {
    async fn get_all(&self, params: &dyn QueryParams) -> RepoResult<ResultPaging<User>> {
        let total = self.total();
        let users = self.fetch(params);
        Ok(ResultPaging {
            total: total.await?,
            items: users.await?,
        })
    }

    async fn find(&self, user_id: &str) -> RepoResult<User> {
        use super::schema::users::dsl::{id, users};
        let mut conn = self
            .pool
            .get()
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        let id_filer = user_id.to_string();
        async_pool::run(move || users.filter(id.eq(id_filer)).first::<UserDiesel>(&mut conn))
            .await
            .map_err(|v| DieselRepoError::from(v).into_inner())
            .map(|v| -> User { v.into() })
    }

    async fn find_by_email(&self, user_email: &str) -> RepoResult<User> {
        use super::schema::users::dsl::{email, users};
        let mut conn = self
            .pool
            .get()
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        let user_email_u = user_email.to_string();
        async_pool::run(move || {
            users
                .filter(email.eq(user_email_u))
                .first::<UserDiesel>(&mut conn)
        })
        .await
        .map_err(|v| DieselRepoError::from(v).into_inner())
        .map(|v| -> User { v.into() })
    }

    async fn create(&self, new_user: &User) -> RepoResult<User> {
        let u: UserDiesel = UserDiesel::from(new_user.clone());
        use super::schema::users::dsl::users;
        let mut conn = self
            .pool
            .get()
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        async_pool::run(move || diesel::insert_into(users).values(u).execute(&mut conn))
            .await
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        Ok(new_user.clone())
    }

    async fn update(&self, user_id: &str, update_user: &UserUpdate) -> RepoResult<User> {
        let u = UserUpdateDiesel::from(update_user.clone());
        use super::schema::users::dsl::{id, users};
        let mut conn = self
            .pool
            .get()
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        let id_filter = user_id.to_string();
        async_pool::run(move || {
            diesel::update(users)
                .filter(id.eq(id_filter))
                .set(u)
                .execute(&mut conn)
        })
        .await
        .map_err(|v| DieselRepoError::from(v).into_inner())?;
        self.find(user_id).await
    }

    async fn delete(&self, user_id: &str) -> RepoResult<()> {
        use super::schema::users::dsl::{id, users};
        let mut conn = self
            .pool
            .get()
            .map_err(|v| DieselRepoError::from(v).into_inner())?;
        let id_filder = user_id.to_string();
        async_pool::run(move || {
            diesel::delete(users)
                .filter(id.eq(id_filder))
                .execute(&mut conn)
        })
        .await
        .map_err(|v| DieselRepoError::from(v).into_inner())?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    fn test_insert() {}
}
