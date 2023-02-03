use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub page_name: String,
    pub action: String,
    pub other_page: String,
    pub other_page_text: String,
}
