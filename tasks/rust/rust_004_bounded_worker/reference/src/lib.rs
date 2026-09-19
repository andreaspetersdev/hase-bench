use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accepted {
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed<R> {
    pub sequence: u64,
    pub result: R,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Undelivered<J> {
    pub sequence: u64,
    pub job: J,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError<J> {
    Full(J),
    Closed(J),
    Failed(J),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerConfigError {
    ZeroCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus<E> {
    Running,
    Closed,
    Failed(E),
}

struct Envelope<J> {
    sequence: u64,
    job: J,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AdmissionStatus {
    Running,
    Closing,
    Closed,
    Failed,
}

struct Admission<J> {
    status: AdmissionStatus,
    next_sequence: u64,
    sender: Option<SyncSender<Envelope<J>>>,
}

struct Outcome<J, R, E> {
    completed: Vec<Completed<R>>,
    undelivered: Vec<Undelivered<J>>,
    failure: Option<E>,
}

pub struct BoundedWorker<J, R, E> {
    admission: Arc<Mutex<Admission<J>>>,
    outcome: Arc<Mutex<Outcome<J, R, E>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl<J, R, E> BoundedWorker<J, R, E> {
    pub fn new<F>(capacity: usize, mut handler: F) -> Result<Self, WorkerConfigError>
    where
        J: Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
        F: FnMut(&J) -> Result<R, E> + Send + 'static,
    {
        if capacity == 0 {
            return Err(WorkerConfigError::ZeroCapacity);
        }
        let (sender, receiver) = mpsc::sync_channel::<Envelope<J>>(capacity);
        let admission = Arc::new(Mutex::new(Admission {
            status: AdmissionStatus::Running,
            next_sequence: 0,
            sender: Some(sender),
        }));
        let outcome = Arc::new(Mutex::new(Outcome {
            completed: Vec::new(),
            undelivered: Vec::new(),
            failure: None,
        }));
        let worker_admission = Arc::clone(&admission);
        let worker_outcome = Arc::clone(&outcome);
        let thread = thread::spawn(move || {
            while let Ok(envelope) = receiver.recv() {
                match handler(&envelope.job) {
                    Ok(result) => lock(&worker_outcome).completed.push(Completed {
                        sequence: envelope.sequence,
                        result,
                    }),
                    Err(error) => {
                        let mut admission = lock(&worker_admission);
                        admission.status = AdmissionStatus::Failed;
                        admission.sender.take();
                        let mut undelivered = vec![Undelivered {
                            sequence: envelope.sequence,
                            job: envelope.job,
                        }];
                        while let Ok(queued) = receiver.try_recv() {
                            undelivered.push(Undelivered {
                                sequence: queued.sequence,
                                job: queued.job,
                            });
                        }
                        let mut outcome = lock(&worker_outcome);
                        outcome.failure = Some(error);
                        outcome.undelivered = undelivered;
                        return;
                    }
                }
            }
            lock(&worker_admission).status = AdmissionStatus::Closed;
        });
        Ok(Self {
            admission,
            outcome,
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn try_submit(&self, job: J) -> Result<Accepted, SubmitError<J>> {
        let mut admission = lock(&self.admission);
        match admission.status {
            AdmissionStatus::Failed => return Err(SubmitError::Failed(job)),
            AdmissionStatus::Closing | AdmissionStatus::Closed => {
                return Err(SubmitError::Closed(job));
            }
            AdmissionStatus::Running => {}
        }
        let sequence = admission.next_sequence;
        let envelope = Envelope { sequence, job };
        match admission
            .sender
            .as_ref()
            .expect("running worker has sender")
            .try_send(envelope)
        {
            Ok(()) => {
                admission.next_sequence += 1;
                Ok(Accepted { sequence })
            }
            Err(TrySendError::Full(envelope)) => Err(SubmitError::Full(envelope.job)),
            Err(TrySendError::Disconnected(envelope)) => {
                admission.status = AdmissionStatus::Closed;
                admission.sender.take();
                Err(SubmitError::Closed(envelope.job))
            }
        }
    }

    pub fn close(&self) {
        {
            let mut admission = lock(&self.admission);
            if admission.status == AdmissionStatus::Running {
                admission.status = AdmissionStatus::Closing;
                admission.sender.take();
            }
        }
        if let Some(thread) = lock(&self.thread).take() {
            let _ = thread.join();
        }
    }

    pub fn status(&self) -> WorkerStatus<E>
    where
        E: Clone,
    {
        match lock(&self.admission).status {
            AdmissionStatus::Running | AdmissionStatus::Closing => WorkerStatus::Running,
            AdmissionStatus::Closed => WorkerStatus::Closed,
            AdmissionStatus::Failed => WorkerStatus::Failed(
                lock(&self.outcome)
                    .failure
                    .as_ref()
                    .expect("failed worker has error")
                    .clone(),
            ),
        }
    }

    pub fn completed(&self) -> Vec<Completed<R>>
    where
        R: Clone,
    {
        lock(&self.outcome).completed.clone()
    }

    pub fn take_undelivered(&self) -> Vec<Undelivered<J>> {
        std::mem::take(&mut lock(&self.outcome).undelivered)
    }
}

impl<J, R, E> Drop for BoundedWorker<J, R, E> {
    fn drop(&mut self) {
        self.close();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
