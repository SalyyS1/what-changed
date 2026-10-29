#![cfg(test)]

#[path = "backpressure/held_operation.rs"]
mod held_operation;

use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use super::fixture::{app_input, capability, Fixture};
use held_operation::HeldOperation;
use what_changed_lib::error::{AppError, DbError};

const GUARD: Duration = Duration::from_secs(30);

#[test]
fn sixteen_queued_writes_reject_the_next_and_retry_without_loss() {
    let fixture = Fixture::new();
    let app = fixture.store.save_app(app_input("Queue capacity")).unwrap();
    let head = HeldOperation::writer(Arc::clone(&fixture.store));
    let (completed, results) = mpsc::channel();
    let mut workers = Vec::new();
    for _ in 0..17 {
        let store = Arc::clone(&fixture.store);
        let input = capability(&app.id);
        let completed = completed.clone();
        workers.push(thread::spawn(move || {
            let result = store.save_capability(input.clone());
            completed.send(result.is_err()).unwrap();
            (input, result)
        }));
    }
    drop(completed);
    // A success cannot finish while the active transaction is held. A full
    // queue acknowledgment therefore establishes that all sixteen slots fill.
    let first_completion = results.recv_timeout(GUARD);
    let head_result = head.finish();
    let joined = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(head_result.unwrap(), 1);
    assert!(first_completion.unwrap());

    let mut accepted = 0;
    let mut rejected = Vec::new();
    for (input, result) in joined {
        match result {
            Ok(saved) => {
                assert_eq!(saved.id, input.id);
                accepted += 1;
            }
            Err(AppError::Db(DbError::QueueFull)) => rejected.push(input),
            Err(error) => panic!("unexpected writer failure: {error}"),
        }
    }
    assert_eq!(accepted, 16);
    assert_eq!(rejected.len(), 1);
    let persisted = fixture.store.list_capabilities(&app.id, false).unwrap();
    assert_eq!(persisted.len(), 16);
    let retry = rejected.pop().unwrap();
    assert!(persisted.iter().all(|saved| saved.id != retry.id));
    let saved = fixture.store.save_capability(retry.clone()).unwrap();
    assert_eq!(fixture.store.save_capability(retry).unwrap(), saved);
    assert_eq!(
        fixture
            .store
            .list_capabilities(&app.id, false)
            .unwrap()
            .len(),
        17
    );
    println!("owner writer saturation: active=1 queued_success=16 rejected_queue_full=1 retry_idempotent=true persisted=17");
}

#[test]
fn two_busy_readers_return_a_bounded_error_then_recover() {
    let fixture = Fixture::new();
    let app = fixture
        .store
        .save_app(app_input("Reader capacity"))
        .unwrap();
    let first = HeldOperation::reader(Arc::clone(&fixture.store));
    let second = HeldOperation::reader(Arc::clone(&fixture.store));
    let store = Arc::clone(&fixture.store);
    let app_id = app.id.clone();
    let (completed, result) = mpsc::channel();
    let waiting = thread::spawn(move || {
        let started = Instant::now();
        let result = store.get_app(&app_id);
        completed.send((started.elapsed(), result)).unwrap();
    });
    let observed = result.recv_timeout(GUARD);
    // Always release and join holders before assertions, including a changed
    // pool deadline. Their own deadlines also bound failed synchronization.
    let first_result = first.finish();
    let second_result = second.finish();
    waiting.join().unwrap();
    assert_eq!(first_result.unwrap(), 1);
    assert_eq!(second_result.unwrap(), 1);
    let (elapsed, result) = observed.unwrap();
    assert!(matches!(
        result,
        Err(AppError::Db(DbError::NotReady { state })) if state == "every reader is busy"
    ));
    assert!(elapsed >= Duration::from_secs(5));
    assert_eq!(fixture.store.get_app(&app.id).unwrap(), Some(app));
    println!(
        "owner reader saturation: held=2 third_not_ready=true elapsed_ms={:.3} recovered=true",
        elapsed.as_secs_f64() * 1000.0
    );
}
