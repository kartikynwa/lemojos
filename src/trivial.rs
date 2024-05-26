// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Html;
use askama::Template;
use rocket::form::{Form, FromForm};
use rocket::http::{Header, Status};
use rocket::response::content::{RawCss, RawJavaScript};
use rocket::response::{status::NoContent, Redirect, Responder};
use rocket::{get, post, uri};

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct Index;

#[get("/")]
pub(crate) fn index() -> Html<Index> {
    Html(Index)
}

#[derive(FromForm)]
pub(crate) struct InstanceForm<'a> {
    instance: &'a str,
}

#[post("/", data = "<form>")]
pub(crate) fn instance_form(form: Form<InstanceForm<'_>>) -> Redirect {
    Redirect::to(uri!(crate::instance(
        form.instance,
    )))
}

#[derive(Responder)]
#[response(content_type = "application/zip")]
pub(crate) struct Code {
    zip: &'static [u8],
    disposition: Header<'static>,
}

#[get("/code")]
pub(crate) fn code() -> Code {
    Code {
        zip: include_bytes!(concat!(env!("OUT_DIR"), "/source.zip")),
        disposition: Header::new(
            "content-disposition",
            r#"attachment; filename="emojos.in.zip""#,
        ),
    }
}

#[get("/static/site.css")]
pub(crate) fn css() -> RawCss<&'static [u8]> {
    RawCss(include_bytes!("site.css"))
}

#[get("/static/copy.js")]
pub(crate) fn copy_js() -> RawJavaScript<&'static [u8]> {
    RawJavaScript(include_bytes!("copy.js"))
}

#[get("/favicon.ico")]
pub(crate) fn favicon_ico() -> Status {
    Status::NotFound
}

#[get("/robots.txt")]
pub(crate) fn robots_txt() -> NoContent {
    NoContent
}
