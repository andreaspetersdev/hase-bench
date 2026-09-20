use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rust_008::{
    BoxFuture, CallError, Clock, ConfigError, Request, RequestHandle, Response, Service,
    SubmitError, Transport, TransportRequest,
};
use tokio::sync::{Notify, mpsc, oneshot};

const WAIT: Duration = Duration::from_secs(3);
type ClockWaiters = Vec<(u64, Arc<Notify>)>;

#[derive(Clone, Default)]
struct ManualClock {
    tick: Arc<AtomicU64>,
    waiters: Arc<Mutex<ClockWaiters>>,
}

impl ManualClock {
    fn at(tick: u64) -> Self {
        let result = Self::default();
        result.tick.store(tick, Ordering::SeqCst);
        result
    }

    fn advance_to(&self, tick: u64) {
        self.tick.store(tick, Ordering::SeqCst);
        let waiters = self.waiters.lock().unwrap();
        for (deadline, notify) in waiters.iter() {
            if *deadline <= tick {
                notify.notify_waiters();
            }
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> u64 {
        self.tick.load(Ordering::SeqCst)
    }

    fn sleep_until(&self, deadline: u64) -> BoxFuture<'static, ()> {
        let clock = self.clone();
        Box::pin(async move {
            loop {
                if clock.now() >= deadline {
                    return;
                }
                let notify = Arc::new(Notify::new());
                let notified = notify.notified();
                clock
                    .waiters
                    .lock()
                    .unwrap()
                    .push((deadline, Arc::clone(&notify)));
                if clock.now() >= deadline {
                    return;
                }
                notified.await;
            }
        })
    }
}

struct Call {
    request: TransportRequest,
    reply: oneshot::Sender<Result<Vec<u8>, &'static str>>,
}

#[derive(Clone)]
struct ControlledTransport {
    calls: mpsc::UnboundedSender<Call>,
    closes: Arc<AtomicUsize>,
    close_error: Option<&'static str>,
}

impl Transport for ControlledTransport {
    type Error = &'static str;

    fn send(&self, request: TransportRequest) -> BoxFuture<'_, Result<Vec<u8>, Self::Error>> {
        let calls = self.calls.clone();
        Box::pin(async move {
            let (reply, result) = oneshot::channel();
            calls.send(Call { request, reply }).unwrap();
            result.await.unwrap_or(Err("send future dropped"))
        })
    }

    fn close(&self) -> BoxFuture<'_, Result<(), Self::Error>> {
        let closes = Arc::clone(&self.closes);
        let error = self.close_error;
        Box::pin(async move {
            closes.fetch_add(1, Ordering::SeqCst);
            match error {
                Some(error) => Err(error),
                None => Ok(()),
            }
        })
    }
}

fn transport(
    close_error: Option<&'static str>,
) -> (
    ControlledTransport,
    mpsc::UnboundedReceiver<Call>,
    Arc<AtomicUsize>,
) {
    let (calls, receiver) = mpsc::unbounded_channel();
    let closes = Arc::new(AtomicUsize::new(0));
    (
        ControlledTransport {
            calls,
            closes: Arc::clone(&closes),
            close_error,
        },
        receiver,
        closes,
    )
}

fn request(stream_id: u64, value: u8, deadline: u64) -> Request {
    Request {
        stream_id,
        payload: vec![value],
        deadline,
    }
}

async fn next_call(receiver: &mut mpsc::UnboundedReceiver<Call>) -> Call {
    tokio::time::timeout(WAIT, receiver.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn wait_until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(WAIT, async move {
        while !predicate() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn public_types_have_required_traits() {
    fn values<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}
    fn send<T: Send>() {}
    fn send_sync<T: Send + Sync>() {}
    values::<ConfigError>();
    values::<SubmitError<&'static str>>();
    values::<CallError<&'static str>>();
    values::<Request>();
    values::<Response>();
    values::<TransportRequest>();
    send::<RequestHandle<&'static str>>();
    send_sync::<Service<&'static str>>();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admission_is_immediate_bounded_and_gap_free() {
    assert!(matches!(
        Service::new(0, transport(None).0, ManualClock::default()),
        Err(ConfigError::ZeroCapacity)
    ));
    if usize::BITS > 32 {
        assert!(matches!(
            Service::new(
                u32::MAX as usize + 1,
                transport(None).0,
                ManualClock::default()
            ),
            Err(ConfigError::CapacityTooLarge)
        ));
    }

    let (transport, mut calls, _) = transport(None);
    let service = Service::new(1, transport, ManualClock::default()).unwrap();
    let first = service.try_start(request(1, 1, 100)).unwrap();
    let rejected = request(2, 2, 100);
    assert!(
        matches!(service.try_start(rejected.clone()), Err(error) if error == SubmitError::Full(rejected))
    );
    let call = next_call(&mut calls).await;
    assert_eq!(call.request.sequence, 0);
    call.reply.send(Ok(vec![11])).unwrap();
    assert_eq!(first.await.unwrap().payload, vec![11]);
    let second = service.try_start(request(2, 2, 100)).unwrap();
    let call = next_call(&mut calls).await;
    assert_eq!(call.request.sequence, 1);
    call.reply.send(Ok(vec![22])).unwrap();
    assert_eq!(second.await.unwrap().sequence, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streams_are_ordered_but_different_streams_make_progress() {
    let (transport, mut calls, _) = transport(None);
    let service = Service::new(3, transport, ManualClock::default()).unwrap();
    let first = service.try_start(request(7, 1, 100)).unwrap();
    let second = service.try_start(request(7, 2, 100)).unwrap();
    let other = service.try_start(request(8, 3, 100)).unwrap();

    let a = next_call(&mut calls).await;
    let b = next_call(&mut calls).await;
    let (same_first, different) = if a.request.stream_id == 7 {
        (a, b)
    } else {
        (b, a)
    };
    assert_eq!(same_first.request.payload, vec![1]);
    assert_eq!(different.request.stream_id, 8);
    assert!(
        calls.try_recv().is_err(),
        "the second same-stream call started too early"
    );

    different.reply.send(Ok(vec![30])).unwrap();
    same_first.reply.send(Ok(vec![10])).unwrap();
    assert_eq!(other.await.unwrap().payload, vec![30]);
    assert_eq!(first.await.unwrap().payload, vec![10]);
    let same_second = next_call(&mut calls).await;
    assert_eq!(same_second.request.sequence, 1);
    assert_eq!(same_second.request.payload, vec![2]);
    same_second.reply.send(Ok(vec![20])).unwrap();
    assert_eq!(second.await.unwrap().payload, vec![20]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_handles_cancels_queued_and_active_work_and_releases_capacity() {
    let (transport, mut calls, _) = transport(None);
    let service = Service::new(2, transport, ManualClock::default()).unwrap();
    let active = service.try_start(request(4, 1, 100)).unwrap();
    let active_call = next_call(&mut calls).await;
    let queued = service.try_start(request(4, 2, 100)).unwrap();
    drop(queued);
    drop(active);
    wait_until(|| service.in_flight() == 0).await;
    assert!(
        active_call.reply.send(Ok(vec![1])).is_err(),
        "active send was not dropped"
    );
    assert!(
        calls.try_recv().is_err(),
        "cancelled queued request reached transport"
    );

    let replacement = service.try_start(request(5, 3, 100)).unwrap();
    let call = next_call(&mut calls).await;
    assert_eq!(call.request.sequence, 2);
    call.reply.send(Ok(vec![3])).unwrap();
    assert_eq!(replacement.await.unwrap().payload, vec![3]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_injected_deadline_covers_queueing_and_active_send() {
    let clock = ManualClock::at(10);
    let (transport, mut calls, _) = transport(None);
    let service = Service::new(3, transport, clock.clone()).unwrap();
    let blocker = service.try_start(request(1, 1, 100)).unwrap();
    let blocker_call = next_call(&mut calls).await;
    let queued = service.try_start(request(1, 2, 20)).unwrap();
    let active = service.try_start(request(2, 3, 20)).unwrap();
    let active_call = next_call(&mut calls).await;

    clock.advance_to(20);
    assert_eq!(queued.await, Err(CallError::DeadlineExceeded));
    assert_eq!(active.await, Err(CallError::DeadlineExceeded));
    assert!(active_call.reply.send(Ok(vec![3])).is_err());
    blocker_call.reply.send(Ok(vec![1])).unwrap();
    blocker.await.unwrap();
    assert!(calls.try_recv().is_err());

    let expired = service.try_start(request(9, 9, 20)).unwrap();
    assert_eq!(expired.await, Err(CallError::DeadlineExceeded));
    assert!(calls.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_send_failure_is_permanent_and_propagates_to_queued_work() {
    let (transport, mut calls, closes) = transport(None);
    let service = Service::new(2, transport, ManualClock::default()).unwrap();
    let first = service.try_start(request(1, 1, 100)).unwrap();
    let queued = service.try_start(request(1, 2, 100)).unwrap();
    let call = next_call(&mut calls).await;
    call.reply.send(Err("broken")).unwrap();
    assert_eq!(first.await, Err(CallError::Transport("broken")));
    assert_eq!(queued.await, Err(CallError::Transport("broken")));
    assert_eq!(service.failure(), Some("broken"));
    let later = request(2, 3, 100);
    assert!(matches!(
        service.try_start(later.clone()),
        Err(error) if error == SubmitError::Failed { request: later, error: "broken" }
    ));
    assert!(calls.try_recv().is_err());
    assert_eq!(service.close().await, Err("broken"));
    assert_eq!(closes.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn close_survives_caller_cancellation_drains_and_closes_transport_once() {
    let (transport, mut calls, closes) = transport(None);
    let service = Arc::new(Service::new(1, transport, ManualClock::default()).unwrap());
    let handle = service.try_start(request(1, 1, 100)).unwrap();
    let call = next_call(&mut calls).await;

    let closing_service = Arc::clone(&service);
    let cancelled_closer = tokio::spawn(async move { closing_service.close().await });
    wait_until(|| service.is_closed()).await;
    cancelled_closer.abort();
    assert_eq!(closes.load(Ordering::SeqCst), 0);
    let rejected = request(2, 2, 100);
    assert!(
        matches!(service.try_start(rejected.clone()), Err(error) if error == SubmitError::Closed(rejected))
    );

    let second_service = Arc::clone(&service);
    let second_closer = tokio::spawn(async move { second_service.close().await });
    call.reply.send(Ok(vec![9])).unwrap();
    assert_eq!(handle.await.unwrap().payload, vec![9]);
    second_closer.await.unwrap().unwrap();
    service.close().await.unwrap();
    assert_eq!(service.in_flight(), 0);
    assert_eq!(closes.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_error_becomes_the_permanent_failure() {
    let (transport, _calls, closes) = transport(Some("close failed"));
    let service = Service::new(1, transport, ManualClock::default()).unwrap();
    assert_eq!(service.close().await, Err("close failed"));
    assert_eq!(service.close().await, Err("close failed"));
    assert_eq!(service.failure(), Some("close failed"));
    assert_eq!(closes.load(Ordering::SeqCst), 1);
}
