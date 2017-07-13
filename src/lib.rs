extern crate futures;

use std::sync::Arc;
use std::sync::atomic::{Ordering,AtomicBool};
use std::thread;
use std::panic;
use std::mem;

use futures::task;
use futures::{Future,Poll,Async};

#[cfg(test)]
mod tests {
    #[test]
    fn test_thread_future() {
        let tf = ThreadFuture::new(|| {
            println!("DOING SOME WORK");
            thread::sleep(Duration::from_millis(500));
            println!("DONE");
            vec![42; 42]
        });
        assert!(tf.wait().unwrap() == vec![42; 42]);
    }
}

enum ThreadFutureInner<F, T> {
    Stopped(F),
    Running(Arc<AtomicBool>, thread::JoinHandle<T>),
    Finished(thread::Result<T>),
    Extracted,
}

/// `ThreadFuture` allows creating a future that waits on the completion of a function executed in
/// a thread
/// 
/// This future will only return `Error` in the event that the thread panics
pub struct ThreadFuture<F, T>(ThreadFutureInner<F, T>);

impl<F, T> ThreadFutureInner<F, T>
where
    F: FnOnce() -> T + panic::UnwindSafe + Send + 'static,
    T: Send + 'static,
{
    fn transition_mut(&mut self) -> Option<thread::Result<T>> {
        use ThreadFutureInner::*;
        let mut state = mem::replace(self, Extracted).transition();
        let mut ret = None;
        state = match state {
            Finished(v) => {
                ret = Some(v);
                Extracted
            }
            o => o,
        };
        mem::replace(self, state);
        ret
    }
    fn transition(self) -> Self {
        use ThreadFutureInner::*;
        match self {
            Stopped(f) => {
                let ready = Arc::new(AtomicBool::new(false));
                let task = task::current();

                let thread_ready = ready.clone();
                let handle = thread::spawn(move || {
                    let ret = panic::catch_unwind(f);
                    thread_ready.store(true, Ordering::Release);
                    task.notify();
                    match ret {
                        Ok(v) => v,
                        Err(e) => panic::resume_unwind(e),
                    }
                });

                Running(ready, handle)
            }
            Running(ready, handle) => {
                match ready.load(Ordering::Acquire) {
                    false => Running(ready, handle),
                    true => Finished(handle.join()),
                }
            }
            v => v,
        }
    }
}

impl<F, T> ThreadFuture<F, T>
where
    F: FnOnce() -> T + panic::UnwindSafe + Send + 'static,
    T: Send + 'static,
{
    /// Create a new `ThreadFuture` that will produce a value once `f` has completed
    ///
    /// Note that f will not begin execution until the first time `poll()` is called on the `ThreadFuture`
    pub fn new(f: F) -> Self {
        ThreadFuture(ThreadFutureInner::Stopped(f))
    }
}

impl<F, T> Future for ThreadFuture<F, T>
where
    F: FnOnce() -> T + panic::UnwindSafe + Send + 'static,
    T: Send + 'static,
{
    type Item = T;
    type Error = Box<std::any::Any + Send>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.transition_mut() {
            Some(s) => s.map(Async::Ready),
            None => Ok(Async::NotReady),
        }
    }
}
