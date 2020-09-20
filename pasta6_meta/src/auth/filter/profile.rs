use crate::{auth::MetaUser, DOMAIN};
use askama_warp::Template;
use pasta6_core::{TemplateContext, User};

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    ctx: TemplateContext<MetaUser>,
}

pub(crate) async fn get_profile(
    current_user: Option<MetaUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ProfileTemplate {
        ctx: TemplateContext::new(current_user, DOMAIN.to_owned()),
    })
}
