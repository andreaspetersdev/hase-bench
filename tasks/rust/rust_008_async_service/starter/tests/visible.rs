use std::sync::{Arc, Mutex};

use rust_008::{BoxFuture, Clock, Request, Service, Transport, TransportRequest};

#[derive(Clone)]
struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> u64 {
        10
    }
    fn sleep_until(&self, _deadline: u64) -> BoxFuture<'static, ()> {
        Box::pin(std::future::pending())
    }
}

#[derive(Clone, Default)]
struct EchoTransport {
    seen: Arc<Mutex<Vec<TransportRequest>>>,
}

impl Transport for EchoTransport {
    type Error = &'static str;

    fn send(&self, request: TransportRequest) -> BoxFuture<'_, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move {
            self.seen.lock().unwrap().push(request.clone());
            Ok(request.payload)
        })
    }

    fn close(&self) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async { Ok(()) })
    }
}

fn request(stream_id: u64, value: u8) -> Request {
    Request {
        stream_id,
        payload: vec![value],
        deadline: 100,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwards_a_request_and_returns_transport_output() {
    let transport = EchoTransport::default();
    let seen = Arc::clone(&transport.seen);
    let service = Service::new(2, transport, FixedClock).unwrap();
    let response = service.try_start(request(7, 42)).unwrap().await.unwrap();
    assert_eq!(response.sequence, 0);
    assert_eq!(response.stream_id, 7);
    assert_eq!(response.payload, vec![42]);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[TransportRequest {
            sequence: 0,
            stream_id: 7,
            payload: vec![42]
        }]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_is_idempotent_and_rejects_new_work() {
    let service = Service::new(1, EchoTransport::default(), FixedClock).unwrap();
    service.close().await.unwrap();
    service.close().await.unwrap();
    assert!(service.is_closed());
    assert!(service.try_start(request(1, 1)).is_err());
}
