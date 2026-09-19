use std::marker::PhantomData;

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

pub struct BoundedWorker<J, R, E> {
    marker: PhantomData<fn(J) -> (R, E)>,
}

impl<J, R, E> BoundedWorker<J, R, E> {
    pub fn new<F>(capacity: usize, _handler: F) -> Result<Self, WorkerConfigError>
    where
        J: Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
        F: FnMut(&J) -> Result<R, E> + Send + 'static,
    {
        if capacity == 0 {
            return Err(WorkerConfigError::ZeroCapacity);
        }
        Ok(Self {
            marker: PhantomData,
        })
    }

    pub fn try_submit(&self, job: J) -> Result<Accepted, SubmitError<J>> {
        Err(SubmitError::Full(job))
    }

    pub fn close(&self) {}

    pub fn status(&self) -> WorkerStatus<E>
    where
        E: Clone,
    {
        WorkerStatus::Running
    }

    pub fn completed(&self) -> Vec<Completed<R>>
    where
        R: Clone,
    {
        Vec::new()
    }

    pub fn take_undelivered(&self) -> Vec<Undelivered<J>> {
        Vec::new()
    }
}
