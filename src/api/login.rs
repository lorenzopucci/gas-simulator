use std::num::NonZeroU32;

use base64::{engine::general_purpose::URL_SAFE, Engine};
use chrono::Duration;
use diesel::{insert_into, prelude::Queryable, ExpressionMethods, QueryDsl};
use ring::{digest, pbkdf2};
use ring::rand::{self, SecureRandom};
use rocket::{http::{HeaderMap, Status}, serde::json::Json};
use rocket_db_pools::{diesel::prelude::RunQueryDsl, Connection};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{api::prop_error, model::{Token, User}, DB};

use super::{ApiError, ApiResponse};

const LEN: usize = digest::SHA512_OUTPUT_LEN;
const SALT_LEN: usize = 16;
const TOKEN_LEN: usize = 256;
const ITER: NonZeroU32 = NonZeroU32::new(600_000).unwrap();

#[derive(Deserialize, Validate)]
pub struct SignupDataForm<'r> {
    username: &'r str,
    #[validate(length(min = 8))]
    password: &'r str,
    #[validate(email)]
    email: &'r str,
}

#[derive(Deserialize)]
pub struct LoginDataForm<'r> {
    username: &'r str,
    password: &'r str,
    duration: Option<u16>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Queryable)]
struct PasswordVerification {
    id: i32,
    password_hash: Vec<u8>,
    salt: Vec<u8>,
}

#[post("/signup", format = "application/json", data = "<signup_data>")]
pub async fn signup(
    signup_data: Json<SignupDataForm<'_>>, mut db: Connection<DB>
) -> Result<ApiResponse<()>, ApiResponse<ApiError>> {
    use crate::schema::users;

    signup_data
        .validate()
        .map_err(|err| prop_error(
            err,
            Status::UnprocessableEntity,
            "Assicurati di inserire un'indirizzo mail valido e una password di almeno 8 caratteri"
        ))?;

    let rng = rand::SystemRandom::new();

    let mut salt = [0; SALT_LEN];
    rng.fill(&mut salt).map_err(|err| prop_error(err, Status::InternalServerError, "Errore durante l'iscrizione"))?;

    let mut hash = [0; LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        ITER,
        &salt,
        signup_data.password.as_bytes(),
        &mut hash,
    );

    insert_into(users::dsl::users)
        .values(User {
            username: signup_data.username.to_string(),
            email: signup_data.email.to_string(),
            password_hash: hash.to_vec(),
            salt: salt.to_vec(),
        })
        .execute(&mut **db)
        .await
        .map_err(|err| prop_error(err, Status::Conflict, "Username già esistente"))?;

    Ok(ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    })
}

#[post("/login", format = "application/json", data = "<login_data>")]
pub async fn login(
    login_data: Json<LoginDataForm<'_>>, mut db: Connection<DB>
) -> Result<ApiResponse<LoginResponse>, ApiResponse<ApiError>> {
    use crate::schema::{users, tokens};

    let rng = rand::SystemRandom::new();

    let user = users::dsl::users
        .select((users::id, users::password_hash, users::salt))
        .filter(users::username.eq(login_data.username))
        .load::<PasswordVerification>(&mut **db)
        .await
        .map_err(|err| prop_error(err, Status::InternalServerError, "Errore durante il login"))?;

    let Some(user) = user.into_iter().next() else {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "Utente non trovato".to_string() },
            headers: HeaderMap::new(),
        });
    };

    pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        ITER,
        &user.salt,
        login_data.password.as_bytes(),
        &user.password_hash
    )
    .map_err(|err| prop_error(err, Status::Unauthorized, "Username o password errati"))?;

    let mut token = [0; TOKEN_LEN];
    rng.fill(&mut token).map_err(|err| prop_error(err, Status::InternalServerError, "Errore durante il login"))?;

    let token = URL_SAFE.encode(token);

    let duration = Duration::minutes(login_data.duration.unwrap_or(60) as i64);

    insert_into(tokens::dsl::tokens)
        .values(Token {
            user_id: user.id,
            token: token.clone(),
            expires: chrono::offset::Utc::now() + duration,
        })
        .execute(&mut **db)
        .await
        .map_err(|err| prop_error(err, Status::InternalServerError, "Errore durante il login"))?;

    Ok(ApiResponse {
        status: Status::Ok,
        body: LoginResponse { token },
        headers: HeaderMap::new(),
    })
}
