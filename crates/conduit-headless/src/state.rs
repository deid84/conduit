use conduit_core::{connection::ConnectionId, Connection};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
