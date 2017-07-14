ThreadFuture
============
`ThreadFuture` allows creating a future that waits on the completion of a function executed in a thread
 
This future will only return `Error` in the event that the thread panics

Example
-------
```rust
use std::time::Duration;
use std::thread::sleep;

fn main() {
    let fut = ThreadFuture::new(|| {
        // Do some work
	sleep(Duration::from_millis(500));
	return vec![42; 42]
    }
    assert!(fut.wait().unwrap() == vec![42; 42]);
}
```
