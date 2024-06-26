// SPDX-License-Identifier: AGPL-3.0-or-later

#![warn(clippy::pedantic)]
#![allow(clippy::needless_pass_by_value, clippy::no_effect_underscore_binding)]

mod trivial;

use askama::Template;
use reqwest::{Client, StatusCode, Url};
use rocket::http::{Header, Status};
use rocket::response::{self, Debug, Responder};
use rocket::{get, routes, Request, Response, State};
use serde::Deserialize;
use std::io::Cursor;
use std::str::FromStr;

#[rocket::launch]
fn rocket() -> _ {
    let client = Client::builder()
        .user_agent(format!(
            "emojos.in/{} (+https://github.com/iliana/emojos.in)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap();

    rocket::build().manage(client).mount(
        "/",
        routes![
            instance,
            trivial::code,
            trivial::copy_js,
            trivial::css,
            trivial::favicon_ico,
            trivial::index,
            trivial::instance_form,
            trivial::robots_txt,
        ],
    )
}

struct Html<T: Template>(T);

impl<T: Template> Responder<'_, 'static> for Html<T> {
    fn respond_to(self, _request: &Request<'_>) -> response::Result<'static> {
        let data = self.0.render().map_err(|_| Status::InternalServerError)?;
        Response::build()
            .header(Header::new("content-type", T::MIME_TYPE))
            .sized_body(data.len(), Cursor::new(data))
            .ok()
    }
}

#[derive(Deserialize)]
struct Site {
    custom_emojis: Vec<EmojoWrapper>,
}

#[derive(Deserialize)]
struct EmojoWrapper {
    custom_emoji: Emojo,
}

#[derive(Deserialize)]
struct Emojo {
    shortcode: String,
    alt_text: String,
    #[serde(rename(deserialize = "image_url"))]
    url: String,
}

#[derive(Template)]
#[template(path = "emojo.html")]
struct Output {
    instance: String,
    emojo: Vec<Emojo>,
}

#[get("/<instance>")]
async fn instance(
    client: &State<Client>,
    instance: String,
) -> Result<Html<Output>, InstanceError> {
    let mut url = Url::from_str("https://host.invalid/api/v3/site").unwrap();
    if url.set_host(Some(&instance)).is_err() {
        return Err(InstanceError::from_kind(Kind::NotFound, instance));
    }

    let emojo_wrappers = match client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => match response.json::<Site>().await {
            Ok(site) => site.custom_emojis,
            Err(err) => return Err(InstanceError::new(err, instance)),
        },
        Err(err) => return Err(InstanceError::new(err, instance)),
    };

    let emojo = emojo_wrappers.into_iter().map(|ew| ew.custom_emoji).collect();

    Ok(Html(Output {
        instance,
        emojo,
    }))
}

#[derive(Template)]
#[template(path = "oh_no.html")]
struct ErrorDisplay {
    status: Status,
    instance: String,
    kind: Kind,
}

#[derive(Responder)]
enum InstanceError {
    Display((Status, Html<ErrorDisplay>)),
    Debug(Debug<reqwest::Error>),
}

impl InstanceError {
    fn new(err: reqwest::Error, instance: String) -> InstanceError {
        let kind = match err.status() {
            Some(StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => Kind::Private,
            Some(StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED | StatusCode::GONE) => {
                Kind::NotFound
            }
            Some(_) => return InstanceError::Debug(Debug(err)),
            None => {
                if err.is_connect() {
                    Kind::NotFound
                } else if err.is_decode() || err.is_redirect() {
                    Kind::Malformed
                } else if err.is_timeout() {
                    Kind::TimedOut
                } else {
                    return InstanceError::Debug(Debug(err));
                }
            }
        };
        InstanceError::from_kind(kind, instance)
    }

    fn from_kind(kind: Kind, instance: String) -> InstanceError {
        InstanceError::from(ErrorDisplay {
            status: match kind {
                Kind::Malformed => Status::BadGateway,
                Kind::NotFound => Status::NotFound,
                Kind::Private => Status::Forbidden,
                Kind::TimedOut => Status::GatewayTimeout,
            },
            instance,
            kind,
        })
    }
}

impl From<ErrorDisplay> for InstanceError {
    fn from(display: ErrorDisplay) -> InstanceError {
        InstanceError::Display((display.status, Html(display)))
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Malformed,
    NotFound,
    Private,
    TimedOut,
}

impl Kind {
    fn message(self) -> &'static str {
        match self {
            Kind::Malformed => "Response from the instance was malformed",
            Kind::NotFound => {
                "Not a fediverse instance, or does not support the Mastodon custom emoji API"
            }
            Kind::Private => "Instance emoji list is private",
            Kind::TimedOut => "Timed out waiting for response",
        }
    }
}
