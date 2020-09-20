use crate::{auth::MetaUser, CONFIG};
use askama_warp::Template;
use pasta6_core::{Config, CoreConfig, TemplateContext, User};

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate<'a> {
    ctx: TemplateContext<'a, CoreConfig, MetaUser>,
}

pub(crate) async fn get_profile(
    current_user: Option<MetaUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ProfileTemplate {
        ctx: TemplateContext::new(&*CONFIG, current_user),
    })
}
