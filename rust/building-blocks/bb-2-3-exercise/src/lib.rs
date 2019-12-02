use bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    #[serde(rename = "_id")] // Use MongoDB's special primary key field name when serializing
    pub id: bson::oid::ObjectId,
    pub name: String,
    pub age: i32,
}
