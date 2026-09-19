use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

use rust_004::{
    Accepted, BoundedWorker, Completed, SubmitError, Undelivered, WorkerConfigError, WorkerStatus,
};

fn assert_copy_traits<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq>() {}
fn assert_value_traits<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}

#[test]
fn public_value_types_keep_the_required_traits() {
    assert_copy_traits::<Accepted>();
    assert_copy_traits::<WorkerConfigError>();
    assert_value_traits::<Completed<String>>();
    assert_value_traits::<WorkerStatus<String>>();
    let undelivered = Undelivered {
        sequence: 4,
        job: Box::new(9),
    };
    assert_eq!(undelivered.job, Box::new(9));
}

#[test]
fn capacity_excludes_in_flight_work_and_rejection_does_not_consume_sequence() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = BoundedWorker::new(1, move |job: &i32| {
        started_tx.send(*job).unwrap();
        release_rx.recv().unwrap();
        Ok::<i32, ()>(*job * 10)
    })
    .unwrap();

    assert_eq!(worker.try_submit(1).unwrap().sequence, 0);
    assert_eq!(started_rx.recv().unwrap(), 1);
    assert_eq!(worker.try_submit(2).unwrap().sequence, 1);
    assert_eq!(worker.try_submit(3), Err(SubmitError::Full(3)));
    release_tx.send(()).unwrap();
    assert_eq!(started_rx.recv().unwrap(), 2);
    assert_eq!(worker.try_submit(4).unwrap().sequence, 2);
    release_tx.send(()).unwrap();
    assert_eq!(started_rx.recv().unwrap(), 4);
    release_tx.send(()).unwrap();
    worker.close();
    assert_eq!(
        worker.completed(),
        vec![
            Completed {
                sequence: 0,
                result: 10
            },
            Completed {
                sequence: 1,
                result: 20
            },
            Completed {
                sequence: 2,
                result: 40
            },
        ]
    );
}

#[derive(Debug, PartialEq, Eq)]
struct MoveOnly {
    id: usize,
    payload: Box<str>,
}

#[test]
fn concurrent_producers_receive_gap_free_ordered_acceptance() {
    let worker =
        Arc::new(BoundedWorker::new(24, |job: &MoveOnly| Ok::<usize, ()>(job.id * 2)).unwrap());
    let mut producers = Vec::new();
    for id in 0..24 {
        let worker = Arc::clone(&worker);
        producers.push(std::thread::spawn(move || {
            let accepted = worker
                .try_submit(MoveOnly {
                    id,
                    payload: format!("job-{id}").into_boxed_str(),
                })
                .unwrap();
            (accepted.sequence, id)
        }));
    }
    let mut accepted: Vec<_> = producers
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    accepted.sort_unstable();
    worker.close();

    assert_eq!(
        accepted
            .iter()
            .map(|(sequence, _)| *sequence)
            .collect::<Vec<_>>(),
        (0..24).collect::<Vec<_>>()
    );
    let expected: Vec<_> = accepted
        .into_iter()
        .map(|(sequence, id)| Completed {
            sequence,
            result: id * 2,
        })
        .collect();
    assert_eq!(worker.completed(), expected);
}

#[test]
fn handler_failure_recovers_failed_and_queued_jobs_in_order() {
    let (failed_tx, failed_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = BoundedWorker::new(3, move |job: &i32| {
        if *job == 2 {
            failed_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Err("handler failed")
        } else {
            Ok(*job * 10)
        }
    })
    .unwrap();

    assert_eq!(worker.try_submit(1).unwrap().sequence, 0);
    assert_eq!(worker.try_submit(2).unwrap().sequence, 1);
    failed_rx.recv().unwrap();
    assert_eq!(worker.try_submit(3).unwrap().sequence, 2);
    assert_eq!(worker.try_submit(4).unwrap().sequence, 3);
    release_tx.send(()).unwrap();
    worker.close();

    assert_eq!(worker.status(), WorkerStatus::Failed("handler failed"));
    assert_eq!(
        worker.completed(),
        vec![Completed {
            sequence: 0,
            result: 10
        }]
    );
    assert_eq!(
        worker.take_undelivered(),
        vec![
            Undelivered {
                sequence: 1,
                job: 2
            },
            Undelivered {
                sequence: 2,
                job: 3
            },
            Undelivered {
                sequence: 3,
                job: 4
            },
        ]
    );
    assert!(worker.take_undelivered().is_empty());
    assert_eq!(worker.try_submit(5), Err(SubmitError::Failed(5)));
}

#[test]
fn close_drains_and_is_idempotent_even_without_work() {
    let worker = BoundedWorker::new(4, |job: &usize| Ok::<usize, ()>(job + 1)).unwrap();
    for job in 0..4 {
        worker.try_submit(job).unwrap();
    }
    worker.close();
    worker.close();
    assert_eq!(worker.status(), WorkerStatus::Closed);
    assert_eq!(worker.completed().len(), 4);
    assert_eq!(worker.try_submit(9), Err(SubmitError::Closed(9)));

    let empty = BoundedWorker::<i32, i32, ()>::new(1, |job| Ok(*job)).unwrap();
    empty.close();
    assert_eq!(empty.status(), WorkerStatus::Closed);
}

#[test]
fn drop_performs_a_draining_close() {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&count);
    {
        let worker = BoundedWorker::new(5, move |_job: &Box<i32>| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok::<(), ()>(())
        })
        .unwrap();
        for job in 0..5 {
            worker.try_submit(Box::new(job)).unwrap();
        }
    }
    assert_eq!(count.load(Ordering::SeqCst), 5);
}
