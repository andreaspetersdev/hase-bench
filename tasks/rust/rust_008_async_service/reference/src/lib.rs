use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::sync::{Mutex as AsyncMutex, Notify, OwnedSemaphorePermit, Semaphore, oneshot};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> u64;
    fn sleep_until(&self, deadline: u64) -> BoxFuture<'static, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub stream_id: u64,
    pub payload: Vec<u8>,
    pub deadline: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRequest {
    pub sequence: u64,
    pub stream_id: u64,
    pub payload: Vec<u8>,
}

pub trait Transport: Send + Sync + 'static {
    type Error: Clone + Send + Sync + 'static;

    fn send(&self, request: TransportRequest) -> BoxFuture<'_, Result<Vec<u8>, Self::Error>>;
    fn close(&self) -> BoxFuture<'_, Result<(), Self::Error>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    ZeroCapacity,
    CapacityTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitError<E> {
    Full(Request),
    Closed(Request),
    Failed { request: Request, error: E },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError<E> {
    Cancelled,
    DeadlineExceeded,
    Transport(E),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub sequence: u64,
    pub stream_id: u64,
    pub payload: Vec<u8>,
}

pub struct RequestHandle<E> {
    result: oneshot::Receiver<Result<Response, CallError<E>>>,
    cancel: Option<oneshot::Sender<()>>,
}

impl<E> Future for RequestHandle<E> {
    type Output = Result<Response, CallError<E>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.result).poll(cx) {
            Poll::Ready(Ok(result)) => {
                self.cancel.take();
                Poll::Ready(result)
            }
            Poll::Ready(Err(_)) => {
                self.cancel.take();
                Poll::Ready(Err(CallError::Cancelled))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<E> Drop for RequestHandle<E> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
}

struct Tail {
    sequence: u64,
    done: Arc<Notify>,
}

struct PendingRequest<E> {
    request: Request,
    sequence: u64,
    predecessor: Option<Arc<Notify>>,
    done: Arc<Notify>,
    permit: OwnedSemaphorePermit,
    cancel: oneshot::Receiver<()>,
    result: oneshot::Sender<Result<Response, CallError<E>>>,
}

struct Inner<E> {
    capacity: u32,
    transport: Arc<dyn Transport<Error = E>>,
    clock: Arc<dyn Clock>,
    permits: Arc<Semaphore>,
    closed: AtomicBool,
    next_sequence: AtomicU64,
    active: AtomicUsize,
    failure: Mutex<Option<E>>,
    streams: Mutex<HashMap<u64, Tail>>,
    close_started: AtomicBool,
    close_result: AsyncMutex<Option<Result<(), E>>>,
    close_done: Notify,
}

pub struct Service<E> {
    inner: Arc<Inner<E>>,
}

impl<E: Clone + Send + Sync + 'static> Service<E> {
    pub fn new<T: Transport<Error = E>, C: Clock>(
        capacity: usize,
        transport: T,
        clock: C,
    ) -> Result<Self, ConfigError> {
        if capacity == 0 {
            return Err(ConfigError::ZeroCapacity);
        }
        let capacity = u32::try_from(capacity).map_err(|_| ConfigError::CapacityTooLarge)?;
        Ok(Self {
            inner: Arc::new(Inner {
                capacity,
                transport: Arc::new(transport),
                clock: Arc::new(clock),
                permits: Arc::new(Semaphore::new(capacity as usize)),
                closed: AtomicBool::new(false),
                next_sequence: AtomicU64::new(0),
                active: AtomicUsize::new(0),
                failure: Mutex::new(None),
                streams: Mutex::new(HashMap::new()),
                close_started: AtomicBool::new(false),
                close_result: AsyncMutex::new(None),
                close_done: Notify::new(),
            }),
        })
    }

    pub fn try_start(&self, request: Request) -> Result<RequestHandle<E>, SubmitError<E>> {
        if let Some(error) = self.failure() {
            return Err(SubmitError::Failed { request, error });
        }
        if self.is_closed() {
            return Err(SubmitError::Closed(request));
        }
        let permit = match Arc::clone(&self.inner.permits).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                if let Some(error) = self.failure() {
                    return Err(SubmitError::Failed { request, error });
                }
                if self.is_closed() {
                    return Err(SubmitError::Closed(request));
                }
                return Err(SubmitError::Full(request));
            }
        };
        if let Some(error) = self.failure() {
            return Err(SubmitError::Failed { request, error });
        }
        if self.is_closed() {
            return Err(SubmitError::Closed(request));
        }

        let sequence = self.inner.next_sequence.fetch_add(1, Ordering::Relaxed);
        let done = Arc::new(Notify::new());
        let predecessor = self
            .inner
            .streams
            .lock()
            .unwrap()
            .insert(
                request.stream_id,
                Tail {
                    sequence,
                    done: Arc::clone(&done),
                },
            )
            .map(|tail| tail.done);
        self.inner.active.fetch_add(1, Ordering::Release);
        let (result_tx, result) = oneshot::channel();
        let (cancel, cancel_rx) = oneshot::channel();
        let inner = Arc::clone(&self.inner);
        tokio::spawn(run_request(
            inner,
            PendingRequest {
                request,
                sequence,
                predecessor,
                done,
                permit,
                cancel: cancel_rx,
                result: result_tx,
            },
        ));
        Ok(RequestHandle {
            result,
            cancel: Some(cancel),
        })
    }

    pub fn in_flight(&self) -> usize {
        self.inner.active.load(Ordering::Acquire)
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn failure(&self) -> Option<E> {
        self.inner.failure.lock().unwrap().clone()
    }

    pub async fn close(&self) -> Result<(), E> {
        self.inner.closed.store(true, Ordering::Release);
        if !self.inner.close_started.swap(true, Ordering::AcqRel) {
            let inner = Arc::clone(&self.inner);
            tokio::spawn(async move { finish_close(inner).await });
        }
        loop {
            let notified = self.inner.close_done.notified();
            if let Some(result) = self.inner.close_result.lock().await.clone() {
                return result;
            }
            notified.await;
        }
    }
}

async fn run_request<E: Clone + Send + Sync + 'static>(
    inner: Arc<Inner<E>>,
    pending: PendingRequest<E>,
) {
    let PendingRequest {
        request,
        sequence,
        predecessor,
        done,
        permit,
        mut cancel,
        result: result_tx,
    } = pending;
    let stream_id = request.stream_id;
    let deadline_tick = request.deadline;
    let mut deadline = inner.clock.sleep_until(deadline_tick);

    let result = if inner.clock.now() >= deadline_tick {
        Err(CallError::DeadlineExceeded)
    } else if let Some(predecessor) = predecessor {
        tokio::select! {
            biased;
            _ = &mut cancel => Err(CallError::Cancelled),
            _ = deadline.as_mut() => Err(CallError::DeadlineExceeded),
            _ = predecessor.notified() => execute_send(&inner, request, sequence, &mut cancel, deadline.as_mut()).await,
        }
    } else {
        execute_send(&inner, request, sequence, &mut cancel, deadline.as_mut()).await
    };

    {
        let mut streams = inner.streams.lock().unwrap();
        if streams
            .get(&stream_id)
            .is_some_and(|tail| tail.sequence == sequence)
        {
            streams.remove(&stream_id);
        }
    }
    done.notify_one();
    inner.active.fetch_sub(1, Ordering::Release);
    drop(permit);
    let _ = result_tx.send(result);
}

async fn execute_send<E: Clone + Send + Sync + 'static>(
    inner: &Arc<Inner<E>>,
    request: Request,
    sequence: u64,
    cancel: &mut oneshot::Receiver<()>,
    deadline: Pin<&mut (dyn Future<Output = ()> + Send)>,
) -> Result<Response, CallError<E>> {
    if let Some(error) = inner.failure.lock().unwrap().clone() {
        return Err(CallError::Transport(error));
    }
    if inner.clock.now() >= request.deadline {
        return Err(CallError::DeadlineExceeded);
    }
    let stream_id = request.stream_id;
    let transport_request = TransportRequest {
        sequence,
        stream_id,
        payload: request.payload,
    };
    let send = inner.transport.send(transport_request);
    tokio::pin!(send);
    tokio::select! {
        biased;
        _ = cancel => Err(CallError::Cancelled),
        _ = deadline => Err(CallError::DeadlineExceeded),
        result = &mut send => match result {
            Ok(payload) => Ok(Response { sequence, stream_id, payload }),
            Err(error) => {
                let permanent = {
                    let mut failure = inner.failure.lock().unwrap();
                    failure.get_or_insert_with(|| error.clone()).clone()
                };
                Err(CallError::Transport(permanent))
            }
        }
    }
}

async fn finish_close<E: Clone + Send + Sync + 'static>(inner: Arc<Inner<E>>) {
    let all_permits = Arc::clone(&inner.permits)
        .acquire_many_owned(inner.capacity)
        .await
        .expect("the service never closes its semaphore");
    let earlier_failure = inner.failure.lock().unwrap().clone();
    let close_result = inner.transport.close().await;
    let result = match (earlier_failure, close_result) {
        (Some(error), _) => Err(error),
        (None, Ok(())) => Ok(()),
        (None, Err(error)) => {
            let permanent = {
                let mut failure = inner.failure.lock().unwrap();
                failure.get_or_insert_with(|| error.clone()).clone()
            };
            Err(permanent)
        }
    };
    *inner.close_result.lock().await = Some(result);
    drop(all_permits);
    inner.close_done.notify_waiters();
}
