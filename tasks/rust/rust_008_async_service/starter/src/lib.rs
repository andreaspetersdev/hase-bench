use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use tokio::sync::{Semaphore, oneshot};

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

pub struct Service<E> {
    closed: AtomicBool,
    next_sequence: AtomicU64,
    active: Arc<AtomicUsize>,
    permits: Arc<Semaphore>,
    _error: std::marker::PhantomData<fn() -> E>,
}

impl<E: Clone + Send + Sync + 'static> Service<E> {
    pub fn new<T: Transport<Error = E>, C: Clock>(
        capacity: usize,
        _transport: T,
        _clock: C,
    ) -> Result<Self, ConfigError> {
        if capacity == 0 {
            return Err(ConfigError::ZeroCapacity);
        }
        if capacity > u32::MAX as usize {
            return Err(ConfigError::CapacityTooLarge);
        }
        Ok(Self {
            closed: AtomicBool::new(false),
            next_sequence: AtomicU64::new(0),
            active: Arc::new(AtomicUsize::new(0)),
            permits: Arc::new(Semaphore::new(capacity)),
            _error: std::marker::PhantomData,
        })
    }

    pub fn try_start(&self, request: Request) -> Result<RequestHandle<E>, SubmitError<E>> {
        if self.is_closed() {
            return Err(SubmitError::Closed(request));
        }
        let permit = Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(|_| SubmitError::Full(request.clone()))?;
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        let active = Arc::clone(&self.active);
        active.fetch_add(1, Ordering::Relaxed);
        let (result_tx, result) = oneshot::channel();
        let (cancel, _cancel_rx) = oneshot::channel();
        tokio::spawn(async move {
            let _permit = permit;
            active.fetch_sub(1, Ordering::Relaxed);
            let _ = result_tx.send(Ok(Response {
                sequence,
                stream_id: request.stream_id,
                payload: request.payload,
            }));
        });
        Ok(RequestHandle {
            result,
            cancel: Some(cancel),
        })
    }

    pub fn in_flight(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn failure(&self) -> Option<E> {
        None
    }

    pub async fn close(&self) -> Result<(), E> {
        self.closed.store(true, Ordering::Release);
        Ok(())
    }
}
