use std::sync::mpsc;

use rust_004::{BoundedWorker, Completed, SubmitError, WorkerConfigError, WorkerStatus};

#[test]
fn processes_jobs_and_publishes_results_in_sequence_order() {
    let worker =
        BoundedWorker::new(2, |job: &String| Ok::<usize, &'static str>(job.len())).unwrap();
    assert_eq!(worker.try_submit("a".to_owned()).unwrap().sequence, 0);
    assert_eq!(worker.try_submit("rust".to_owned()).unwrap().sequence, 1);
    worker.close();
    assert_eq!(worker.status(), WorkerStatus::Closed);
    assert_eq!(
        worker.completed(),
        vec![
            Completed {
                sequence: 0,
                result: 1
            },
            Completed {
                sequence: 1,
                result: 4
            }
        ]
    );
}

#[test]
fn full_queue_returns_the_original_job_without_blocking() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut first = true;
    let worker = BoundedWorker::new(1, move |job: &Box<i32>| {
        if first {
            first = false;
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }
        Ok::<i32, ()>(**job)
    })
    .unwrap();
    worker.try_submit(Box::new(1)).unwrap();
    started_rx.recv().unwrap();
    assert_eq!(worker.try_submit(Box::new(2)).unwrap().sequence, 1);
    assert_eq!(
        worker.try_submit(Box::new(3)),
        Err(SubmitError::Full(Box::new(3)))
    );
    release_tx.send(()).unwrap();
    worker.close();
}

#[test]
fn rejects_invalid_capacity_and_submissions_after_repeated_close() {
    let invalid = BoundedWorker::<i32, i32, ()>::new(0, |job| Ok(*job));
    assert!(matches!(invalid, Err(WorkerConfigError::ZeroCapacity)));
    let worker = BoundedWorker::new(1, |job: &i32| Ok::<i32, ()>(*job)).unwrap();
    worker.close();
    worker.close();
    assert_eq!(worker.status(), WorkerStatus::Closed);
    assert_eq!(worker.try_submit(7), Err(SubmitError::Closed(7)));
}
