use std::{collections::HashMap, io, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use tokio::{
    sync::{Notify, RwLock},
    time::timeout,
};

type UniqueId = u32;

/// `WaitingParties` holds the actual waiting party associated with some `UniqueId`.
#[derive(Default)]
struct WaitingParties(HashMap<UniqueId, Arc<Notify>>);

impl WaitingParties {
    fn take(&mut self, unique_id: UniqueId) -> Option<Arc<Notify>> {
        self.0.remove(&unique_id)
    }

    fn insert(&mut self, unique_id: UniqueId) -> Arc<Notify> {
        let waiting_party = Arc::new(Notify::new());
        self.0.insert(unique_id, waiting_party.clone());
        waiting_party
    }

    fn remove(&mut self, unique_id: UniqueId) {
        self.0.remove(&unique_id);
    }
}

#[derive(Default)]
struct AppState {
    waiting_parties: RwLock<WaitingParties>,
}

async fn sync_parties(
    Path(unique_id): Path<UniqueId>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut waiting_parties = state.waiting_parties.write().await;

    if let Some(party) = waiting_parties.take(unique_id) {
        // Simply notify the other waiting party
        party.notify_one();
        ok_response()
    } else {
        // There is no waiting party for this id, so we are the one waiting
        let party = waiting_parties.insert(unique_id);

        // We drop the guard to avoid race condition
        drop(waiting_parties);

        // We will wait patiently up to 10 seconds for someone else to connect
        match timeout(Duration::from_secs(10), party.notified()).await {
            Ok(_) => ok_response(),
            Err(_) => {
                // In case we timed out, we clean up the previously stored waiting party.
                state.waiting_parties.write().await.remove(unique_id);
                timeout_response()
            }
        }
    }
}

fn ok_response() -> Response {
    (
        StatusCode::OK,
        format!("Hooray! We got another party connected!\n"),
    )
        .into_response()
}

fn timeout_response() -> Response {
    (
        StatusCode::REQUEST_TIMEOUT,
        format!("Oh no... we timed out waiting for another party\n"),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let state = Arc::new(AppState::default());

    let app = Router::new()
        .route("/wait-for-second-party/:unique-id", post(sync_parties))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    Ok(axum::serve(listener, app).await?)
}
