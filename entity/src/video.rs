use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "seaorm")]
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Decode, Encode)] // TODO : remove the Debug
#[cfg_attr(feature = "seaorm", derive(DeriveEntityModel))]
#[cfg_attr(feature = "seaorm", sea_orm(table_name = "videos"))]
pub struct Model {
    #[cfg_attr(feature = "seaorm", sea_orm(primary_key, auto_increment = false))]
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub author: String,
    pub duration: String,
}

#[cfg(feature = "seaorm")]
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[cfg(feature = "seaorm")]
impl ActiveModelBehavior for ActiveModel {}
