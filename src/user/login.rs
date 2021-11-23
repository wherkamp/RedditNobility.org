use actix_web::{get, post, web::Json, HttpRequest};
use bcrypt::verify;

use crate::api_response::{APIResponse, SiteResponse};
use crate::error::response::unauthorized;
use crate::{Database, RN};
use serde::{Deserialize, Serialize};

use crate::user::action::{delete_otp, get_opt, get_user_by_id, get_user_by_name};
use crate::user::models::Status;
use crate::user::utils::{create_token, generate_otp, get_user_by_header};
use crate::utils::send_login;

#[get("/api/me")]
pub async fn me(database: Database, request: HttpRequest) -> SiteResponse {
    let connection = database.get()?;

    let user = get_user_by_header(request.headers(), &connection)?;
    if user.is_none() {
        return unauthorized();
    }

    APIResponse::respond_new(user, &request)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[post("/api/login/password")]
pub async fn login(login: Json<Login>, database: Database, request: HttpRequest) -> SiteResponse {
    let connection = database.get()?;
    let option = get_user_by_name(&login.username, &connection)?;
    if option.is_none() {
        return unauthorized();
    }
    let user = option.unwrap();
    if user.status != Status::Approved || !user.permissions.login {
        return unauthorized();
    }
    if verify(&login.password, &user.password)? {
        let x = create_token(&user, &connection)?;
        return APIResponse::new(true, Some(x)).respond(&request);
    }
    return unauthorized();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateOTP {
    pub username: String,
}

#[post("/api/login/otp/create")]
pub async fn one_time_password_create(
    otp_request: Json<CreateOTP>,
    rn: RN,
    database: Database,
    request: HttpRequest,
) -> SiteResponse {
    println!("One");
    let connection = database.get()?;
    let option = get_user_by_name(&otp_request.username, &connection)?;
    if option.is_none() {
        return unauthorized();
    }
    println!("Two");
    let user = option.unwrap();
    if user.status != Status::Approved || !user.permissions.login {
        return unauthorized();
    }
    println!("Three");

    let rn = rn.lock()?;
    println!("Four");
    let string = generate_otp(&user.id, &connection)?;
    send_login(&user.username, string, &rn.reddit)?;
    return APIResponse {
        success: true,
        data: Some(true),
        status_code: Some(201),
    }
        .respond(&request);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UseOTP {
    pub username: String,
    pub otp: String,
}

#[post("/api/login/otp")]
pub async fn one_time_password(
    otp: Json<UseOTP>,
    database: Database,
    request: HttpRequest,
) -> SiteResponse {
    let connection = database.get()?;
    let option = get_opt(&otp.otp, &connection)?;
    if option.is_none() {
        return unauthorized();
    }
    let option = option.unwrap();
    let user = get_user_by_id(&option.user, &connection)?;
    if user.is_none() {
        //Ask questions later???
        return unauthorized();
    }
    let user = user.unwrap();
    if user.status != Status::Approved || !user.permissions.login {
        //Ask questions later???
        return unauthorized();
    }
    delete_otp(option.id, &connection)?;
    let x = create_token(&user, &connection)?;
    return APIResponse::new(true, Some(x)).respond(&request);
}
