use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSelection {
    pub fields: Option<String>, // e.g. "id,name,email"
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub phone_number: Option<String>,
    pub limit: Option<i32>,
    #[serde(rename = "skip")]
    pub off_set: Option<i32>,
}
