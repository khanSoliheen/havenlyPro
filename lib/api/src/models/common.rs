use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FieldSelection {
    pub fields: Option<String>, // e.g. "id,name,email"
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub phone_number: Option<String>,
}
