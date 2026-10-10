//! A small ordered worker pool. Reading (OCR) and the on-device model both gain from a
//! few concurrent requests; progress and cancellation stay on the calling thread.
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};

/// Runs `work` over `items` with up to `workers` threads and returns results in input
/// order. `progress` receives the number finished so far; an error from it stops new
/// work, waits for running items and is returned.
pub fn map<T: Sync, R: Send>(
    items: &[T],
    workers: usize,
    work: impl Fn(&T) -> R + Sync,
    mut progress: impl FnMut(usize) -> Result<(), String>,
) -> Result<Vec<R>, String> {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..workers.clamp(1, items.len().max(1)) {
            let sender = sender.clone();
            let (next, stop, work) = (&next, &stop, &work);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= items.len() || sender.send((index, work(&items[index]))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        let mut results: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
        for (done, (index, result)) in receiver.into_iter().enumerate() {
            results[index] = Some(result);
            if let Err(error) = progress(done + 1) {
                stop.store(true, Ordering::Relaxed);
                return Err(error);
            }
        }
        results
            .into_iter()
            .collect::<Option<Vec<R>>>()
            .ok_or_else(|| "A background worker stopped unexpectedly.".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn results_keep_input_order_and_progress_counts_up() {
        let seen = Mutex::new(Vec::new());
        let result = map(
            &[30u64, 1, 20, 5],
            3,
            |&ms| {
                thread::sleep(std::time::Duration::from_millis(ms));
                ms * 2
            },
            |done| {
                seen.lock().unwrap().push(done);
                Ok(())
            },
        );
        assert_eq!(result.unwrap(), vec![60, 2, 40, 10]);
        assert_eq!(*seen.lock().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn a_progress_error_stops_new_work() {
        let started = AtomicUsize::new(0);
        let result = map(
            &[0; 50],
            2,
            |_| {
                started.fetch_add(1, Ordering::Relaxed);
                thread::sleep(std::time::Duration::from_millis(2));
            },
            |done| {
                if done == 3 {
                    Err("cancelled".into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(result.unwrap_err(), "cancelled");
        assert!(started.load(Ordering::Relaxed) < 50);
        assert!(map(&[] as &[u8], 4, |_| (), |_| Ok(())).unwrap().is_empty());
    }
}
