use diesel::{pg::PgConnection, prelude::*, QueryResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::users;

#[derive(Queryable, Serialize, Insertable, Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

impl User {
    pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
        cfg.service(routes::login);
        cfg.service(routes::register);
    }
}
#[derive(Insertable, Serialize, Deserialize, Clone)]
#[table_name = "users"]
pub struct NewUser {
    username: String,
    password: String,
}

impl NewUser {
    fn hash_password(&self) -> Self {
        Self {
            password: crate::utils::hash_password(&self.password),
            username: self.username.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginUser {
    username: String,
    password: String,
}

#[derive(AsChangeset, Deserialize)]
#[table_name = "users"]
pub struct UpdateUser {
    username: Option<String>,
    password: Option<String>,
}

impl UpdateUser {
    fn hash_password(self) -> Self {
        let password =
            self.password.as_ref().map(|s| crate::utils::hash_password(s));

        Self { password, username: self.username }
    }
}

impl User {
    fn update(
        id: Uuid,
        conn: &PgConnection,
        update_user: UpdateUser,
    ) -> QueryResult<Self> {
        let update_user = update_user.hash_password();

        diesel::update(users::table.filter(users::id.eq(id)))
            .set(update_user)
            .get_result(conn)
    }

    fn create(conn: &PgConnection, newuser: &NewUser) -> QueryResult<Self> {
        let newuser = newuser.hash_password();

        diesel::insert_into(users::table).values(newuser).get_result(conn)
    }

    fn find_user(
        conn: &PgConnection,
        loginuser: &LoginUser,
    ) -> QueryResult<Self> {
        users::table
            .filter(
                users::username.eq(&loginuser.username), //.filter(users::password.eq(&user.password)),
            )
            .load::<User>(conn)?
            .into_iter()
            .find(move |user| user.verify_password(loginuser))
            .ok_or_else(|| diesel::result::Error::NotFound)
    }

    pub fn find_by_id(conn: &PgConnection, id: Uuid) -> QueryResult<Self> {
        users::table.filter(users::id.eq(id)).first::<User>(conn)
    }
}

#[derive(Debug)]
struct InvalidPassword;

impl User {
    fn into_jwt(self) -> crate::utils::jwt::Token {
        use crate::utils::jwt::Jwt;
        use chrono::Duration;

        Jwt::new("journali.nl".to_string(), Duration::days(30), self.id)
            .tokenize()
    }

    fn verify_password(&self, user: &LoginUser) -> bool {
        match bcrypt::verify(&user.password, &self.password) {
            Ok(true) => true,
            _ => false,
        }
    }
}

mod routes {
    use actix_web::{
        patch, post,
        web::{self},
        Error, HttpResponse,
    };

    use crate::utils::responsable::Responsable;
    use crate::{database::exec_on_pool, DbPool};

    use super::{LoginUser, NewUser, UpdateUser, User};
    use uuid::Uuid;

    #[post("/login")]
    pub(super) async fn login(
        pool: web::Data<DbPool>,
        user: web::Json<LoginUser>,
    ) -> Result<HttpResponse, Error> {
        let cloned_user = user.clone();

        exec_on_pool(&pool, move |conn| User::find_user(conn, &cloned_user))
            .await
            .map(User::into_jwt)
            .into_response()
    }

    #[post("/register")]
    pub(super) async fn register(
        pool: web::Data<DbPool>,
        new_user: web::Json<NewUser>,
    ) -> Result<HttpResponse, Error> {
        exec_on_pool(&pool, move |conn| User::create(conn, &new_user))
            .await
            .into_response()
    }

    #[patch("/users/{id}")]
    pub async fn update_todo_item(
        pool: web::Data<DbPool>,
        id: web::Path<Uuid>,
        update_user: web::Json<UpdateUser>,
    ) -> Result<HttpResponse, Error> {
        // Crudder::<TodoItem>::update(id.into_inner(), form.into_inner(), &pool)
        //     .await
        exec_on_pool(&pool, move |conn| {
            User::update(id.into_inner(), conn, update_user.into_inner())
        })
        .await
        .into_response()
    }
}
