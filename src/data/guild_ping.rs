use sea_orm::entity::prelude::*;
use sea_orm::prelude::DateTimeUtc;

// note: we store all u64s as i64 since sqlite doesn't technically support u64
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "GuildPings")]
pub struct Model {
    pub guild_id: i64,
    pub user_id: i64,
    pub last_everyone_ping: Option<DateTimeUtc>,
    pub last_here_ping: Option<DateTimeUtc>,
    pub last_role_ping: Option<DateTimeUtc>,
    pub last_user_ping: Option<DateTimeUtc>,
    pub pings: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    GuildId,
    UserId,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (i64, i64);

    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
