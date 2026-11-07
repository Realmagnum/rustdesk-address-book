pub mod ab;
pub mod auth;
pub mod frontend;
pub mod groups;
pub mod peers;
pub mod system;
pub mod tags;
pub mod users;
pub mod device_compat;

use axum::Router;
use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(auth::routes())
        .merge(ab::routes())
        .merge(peers::routes())
        .merge(tags::routes())
        .merge(system::routes())
        .merge(users::routes())
        .merge(groups::routes())
	.merge(device_compat::routes())
}
