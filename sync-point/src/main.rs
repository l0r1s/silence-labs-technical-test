use std::{collections::HashMap, io, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use tokio::{
    sync::{Notify, RwLock},
    time::timeout,
};

static INBOUND_MESSAGE: &str = "Hooray! Another party is connected!\n";
static OUTBOUND_MESSAGE: &str = "Yippee! We connected to another party!\n";
static TIMEOUT_MESSAGE: &str = "Oh no... we timed out waiting for another party\n";

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
    wait_timeout: Duration,
    waiting_parties: RwLock<WaitingParties>,
}

impl AppState {
    fn new(wait_timeout: Duration) -> Self {
        AppState {
            wait_timeout,
            waiting_parties: Default::default(),
        }
    }
}

async fn sync_parties(
    Path(unique_id): Path<UniqueId>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut waiting_parties = state.waiting_parties.write().await;

    if let Some(party) = waiting_parties.take(unique_id) {
        // Simply notify the other waiting party
        party.notify_one();
        (StatusCode::OK, OUTBOUND_MESSAGE.to_string()).into_response()
    } else {
        // There is no waiting party for this id, so we are the one waiting
        let party = waiting_parties.insert(unique_id);

        // We drop the guard to avoid race condition
        drop(waiting_parties);

        // We will wait patiently up to 10 seconds for someone else to connect
        match timeout(state.wait_timeout, party.notified()).await {
            Ok(_) => (StatusCode::OK, INBOUND_MESSAGE.to_string()).into_response(),
            Err(_) => {
                // In case we timed out, we clean up the previously stored waiting party.
                state.waiting_parties.write().await.remove(unique_id);
                (StatusCode::REQUEST_TIMEOUT, TIMEOUT_MESSAGE.to_string()).into_response()
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let (app, _state) = make_app(Duration::from_secs(10));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    Ok(axum::serve(listener, app).await?)
}

fn make_app(wait_duration: Duration) -> (Router, Arc<AppState>) {
    let state = Arc::new(AppState::new(wait_duration));

    (
        Router::new()
            .route("/wait-for-second-party/:unique-id", post(sync_parties))
            .with_state(state.clone()),
        state,
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use axum::{
        body::{Body, Bytes},
        http::Request,
        response::Response,
        routing::{future::RouteFuture, RouterIntoService},
    };
    use http_body_util::BodyExt;
    use tokio::time::sleep;
    use tower::{Service, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn two_parties_with_same_id_succeed() {
        let (app, _state) = make_app(Duration::from_millis(200));
        let mut app = app.into_service();

        let party1_request = make_test_request(1);
        let party1_response = run_request(&mut app, party1_request).await;

        let party2_request = make_test_request(1);
        let party2_response = run_request(&mut app, party2_request).await;

        let (party1_response, party2_response) = tokio::join!(party1_response, party2_response);
        let (party1_response, party2_response) =
            (party1_response.unwrap(), party2_response.unwrap());

        assert_eq!(party1_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party1_response).await[..],
            INBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party2_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party2_response).await[..],
            OUTBOUND_MESSAGE.as_bytes()
        );
    }

    #[tokio::test]
    async fn single_party_time_out() {
        let (app, _state) = make_app(Duration::from_millis(100));
        let mut app = app.into_service();

        let party1_request = make_test_request(1);
        let party1_response = run_request(&mut app, party1_request).await.await.unwrap();

        sleep(Duration::from_millis(150)).await;

        assert_eq!(party1_response.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            &extract_response_body(party1_response).await[..],
            TIMEOUT_MESSAGE.as_bytes()
        );
    }

    #[tokio::test]
    async fn multiple_parties_with_multiple_ids_succeed() {
        let (app, _state) = make_app(Duration::from_millis(200));
        let mut app = app.into_service();

        let party1_request = make_test_request(1);
        let party1_response = run_request(&mut app, party1_request).await;

        let party2_request = make_test_request(1);
        let party2_response = run_request(&mut app, party2_request).await;

        let party3_request = make_test_request(2);
        let party3_response = run_request(&mut app, party3_request).await;

        let party4_request = make_test_request(2);
        let party4_response = run_request(&mut app, party4_request).await;

        let (party1_response, party2_response, party3_response, party4_response) = tokio::join!(
            party1_response,
            party2_response,
            party3_response,
            party4_response
        );
        let (party1_response, party2_response, party3_response, party4_response) = (
            party1_response.unwrap(),
            party2_response.unwrap(),
            party3_response.unwrap(),
            party4_response.unwrap(),
        );

        assert_eq!(party1_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party1_response).await[..],
            INBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party2_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party2_response).await[..],
            OUTBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party3_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party3_response).await[..],
            INBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party4_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party4_response).await[..],
            OUTBOUND_MESSAGE.as_bytes()
        );
    }

    #[tokio::test]
    async fn multiple_parties_with_multiple_ids_some_succeed_some_timeout() {
        let (app, _state) = make_app(Duration::from_millis(200));
        let mut app = app.into_service();

        let party1_request = make_test_request(1);
        let party1_response = run_request(&mut app, party1_request).await;

        let party2_request = make_test_request(1);
        let party2_response = run_request(&mut app, party2_request).await;

        let party3_request = make_test_request(2);
        let party3_response = run_request(&mut app, party3_request).await;

        let party4_request = make_test_request(2);
        let party4_response = run_request(&mut app, party4_request).await;

        let party5_request = make_test_request(2);
        let party5_response = run_request(&mut app, party5_request).await;

        let party6_request = make_test_request(3);
        let party6_response = run_request(&mut app, party6_request).await;

        let (
            party1_response,
            party2_response,
            party3_response,
            party4_response,
            party5_response,
            party6_response,
        ) = tokio::join!(
            party1_response,
            party2_response,
            party3_response,
            party4_response,
            party5_response,
            party6_response,
        );
        let (
            party1_response,
            party2_response,
            party3_response,
            party4_response,
            party5_response,
            party6_response,
        ) = (
            party1_response.unwrap(),
            party2_response.unwrap(),
            party3_response.unwrap(),
            party4_response.unwrap(),
            party5_response.unwrap(),
            party6_response.unwrap(),
        );

        assert_eq!(party1_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party1_response).await[..],
            INBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party2_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party2_response).await[..],
            OUTBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party3_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party3_response).await[..],
            INBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party4_response.status(), StatusCode::OK);
        assert_eq!(
            &extract_response_body(party4_response).await[..],
            OUTBOUND_MESSAGE.as_bytes()
        );

        assert_eq!(party5_response.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            &extract_response_body(party5_response).await[..],
            TIMEOUT_MESSAGE.as_bytes()
        );

        assert_eq!(party6_response.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            &extract_response_body(party6_response).await[..],
            TIMEOUT_MESSAGE.as_bytes()
        );
    }

    fn make_test_request(unique_id: UniqueId) -> Request<Body> {
        Request::builder()
            .uri(format!("/wait-for-second-party/{}", unique_id))
            .method("POST")
            .body(Body::empty())
            .expect("creating fake request with empty body shouldn't fail")
    }

    async fn extract_response_body(response: Response) -> Bytes {
        response.into_body().collect().await.unwrap().to_bytes()
    }

    async fn run_request(
        app: &mut RouterIntoService<Body>,
        request: Request<Body>,
    ) -> RouteFuture<Infallible> {
        ServiceExt::<Request<Body>>::ready(app)
            .await
            .unwrap()
            .call(request)
    }
}
