use std::path::Path;
use actix_web::{get, web, HttpRequest};

use crate::api_response::{APIResponse, SiteResponse};
use crate::{Database, User, RN, utils};
use new_rawr::structures::submission::Submission;
use new_rawr::traits::{Content, Votable};

use crate::error::response::{bad_request, error, not_found, unauthorized};
use crate::user::action::{get_found_users, get_user_by_name, update_properties};
use crate::user::utils::get_user_by_header;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use actix_web::error::ParseError::Status;
use actix_web::http::StatusCode;
use actix_web::web::Json;
use strum::ParseError;
use crate::user::models::{Level, Status};
use crate::user::models::Level::User;
use crate::user::models::Status::Approved;
use crate::utils::get_current_time;

#[get("/moderator/user/{user}")]
pub async fn user_page(
    database: Database,
    web::Path(username): web::Path<String>,
    req: HttpRequest,
) -> SiteResponse {
    let connection = database.get()?;
    let user = get_user_by_header(req.headers(), &connection)?;
    if user.is_none() {
        return unauthorized();
    }
    let user = user.unwrap();
    if user.level == Level::User {
        return unauthorized();
    }
    let lookup = get_user_by_name(&username, &connection)?;
    return APIResponse::<User>::respond_new(lookup, &req);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedditUser {
    pub name: String,
    pub avatar: String,
    pub commentKarma: i64,
    pub total_karma: i64,
    pub created: i64,
    pub topFivePosts: Vec<RedditPost>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedditPost {
    pub subreddit: String,
    pub url: String,
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: i64,
}

#[get("/api/moderator/review/{user}")]
pub async fn review_user(
    database: Database,
    web::Path(username): web::Path<String>,
    req: HttpRequest,
    rr: RN,
) -> SiteResponse {
    let conn = database.get()?;
    let user = get_user_by_header(req.headers(), &conn)?;
    if user.is_none() {
        return unauthorized();
    }
    let user = user.unwrap();
    if user.level == Level::User {
        return unauthorized();
    }
    let mut rn = rr.lock()?;
    let user = if username.eq("next") {
        let mut result = get_found_users(&conn)?;
        result.sort_by_key(|x| x.created);
        let mut v = None;
        for i in 0..result.len() {
            let user = result.remove(i);
            if !rn.users_being_worked_on.contains_key(&user.id) {
                v = Some(user);
                break;
            }
        }
        if v.is_none() {
            return not_found();
        }
        v.unwrap()
    } else {
        let user = get_user_by_name(&username, &conn)?;
        if user.is_none() {
            return not_found();
        }
        user.unwrap()
    };

    rn.add_id(user.id);
    let r_user = rn.reddit.user(&user.username);
    let about = rn.reddit.user(&user.username).about()?;

    let submissions = r_user
        .submissions()
        .unwrap()
        .take(5)
        .collect::<Vec<Submission>>();
    let mut user_posts = Vec::<RedditPost>::new();

    for x in submissions {
        let post = RedditPost {
            subreddit: x.subreddit().name,
            url: format!("https://reddit.com{}", x.data.permalink),
            id: x.data.id.clone(),
            title: x.data.title.clone(),
            content: x.data.selftext.clone().to_string(),
            score: x.score(),
        };
        user_posts.push(post);
    }
    let user = RedditUser {
        name: about.data.name,
        avatar: about.data.icon_img.unwrap_or("".parse().unwrap()),
        commentKarma: about.data.comment_karma,
        total_karma: about.data.total_karma,
        created: about.data.created as i64,
        topFivePosts: user_posts,
    };
    let response = APIResponse::<RedditUser> {
        success: true,
        data: Some(user),
        status_code: Some(200),
    };
    response.respond(&req)
}

#[get("/api/moderator/review/{user}/{status}")]
pub async fn review_user_update(
    database: Database,
    value: web::Path<(String, String)>,
    req: HttpRequest,
    rn: RN,
) -> SiteResponse {
    let conn = database.get()?;
    let user = get_user_by_header(req.headers(), &conn)?;
    if user.is_none() {
        return unauthorized();
    }
    let user = user.unwrap();
    if user.level == Level::User {
        return unauthorized();
    }
    let option = get_user_by_name(&value.0.0, &conn)?;
    if option.is_none() {
        return not_found();
    }
    let str: Result<Status, ParseError> = Status::from_str(value.0.1.as_str());
    if str.is_err() {
        return bad_request("Approved or Denied".to_string());
    }
    let status = str.unwrap();
    if status == Approved {
        let rr = rn.lock()?;
        let user1 = utils::approve_user(&user, &rr.unwrap().reddit);
        if !user1 {
            return crate::error::response::error("Unable to Process Approve Request Currently", Some(StatusCode::INTERNAL_SERVER_ERROR));
        }
    }
    crate::moderator::action::update_status(&option.unwrap().id, status, &user.username, get_current_time(), &conn)?;
    return APIResponse::new(true, Some(true)).respond(&req);
}


#[derive(serde::Deserialize)]
pub struct ChangeRequest {
    pub value: String,
}

#[post("/api/moderator/update/{user}/{key}")]
pub async fn moderator_update_properties(
    database: Database,
    request: Json<ChangeRequest>,
    value: web::Path<(String, String)>,
    r: HttpRequest,
) -> SiteResponse {
    let conn = database.get()?;
    let option = get_user_by_header(r.headers(), &conn)?;
    if option.is_none() {
        return unauthorized();
    }
    let mut modetator = option.unwrap();
    if modetator.level == User {
        return unauthorized();
    }
    // Update User
    let option = get_user_by_name(&value.0.0, &conn)?;
    if option.is_none() {
        return not_found();
    }
    let mut user = option.unwrap();
    let value = request.0.value;
    match value.0.1.as_str() {
        "avatar" => {
            user.properties.set_avatar(value);
        }
        "description" => {
            user.properties.set_description(value);
        }
        _ => {
            return bad_request("You can only change your Avatar or Description");
        }
    }
    update_properties(&user.id, user.properties, &conn)?;
    return APIResponse::new(true, Some(true)).respond(&r);
}
