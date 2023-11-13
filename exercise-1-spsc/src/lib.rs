#![allow(unused_variables)]

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::atomic::Ordering;
use std::sync::Arc;

// Check if we can tweak the buffer size for performance
const BUFFER_SIZE: usize = 4096;

pub struct Producer<T: Send> {
    message_buffer: Arc<[UnsafeCell<Option<T>>; BUFFER_SIZE]>,
    read_index: Arc<AtomicUsize>,
    write_index: Arc<AtomicUsize>,
    producer_counter: Arc<AtomicUsize>,
    consumer_counter: Arc<AtomicUsize>,
    synchronizer: Arc<AtomicBool>,
    _marker: PhantomData<T>,
}
pub struct Consumer<T: Send> {
    message_buffer: Arc<[UnsafeCell<Option<T>>; BUFFER_SIZE]>,
    read_index: Arc<AtomicUsize>,
    write_index: Arc<AtomicUsize>,
    producer_counter: Arc<AtomicUsize>,
    consumer_counter: Arc<AtomicUsize>,
    synchronizer: Arc<AtomicBool>,
    _marker: PhantomData<T>,
}

pub struct SPSC<T: Send> {
    producer: Producer<T>,
    consumer: Consumer<T>,
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug)]
pub struct RecvError;

impl<T: Send> SPSC<T> {
    const INIT: UnsafeCell<Option<T>> = UnsafeCell::new(None);
    pub fn new() -> Self {
        // The only way I found for 2 threads to share a buffer is unsafe cells
        let cell_array: [UnsafeCell<Option<T>>; BUFFER_SIZE] = [Self::INIT; BUFFER_SIZE];

        let message_buffer: Arc<[UnsafeCell<Option<T>>; BUFFER_SIZE]> = Arc::new(cell_array);

        let read_index: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let write_index: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let producer_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));
        let consumer_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));

        let synchronizer = Arc::new(AtomicBool::new(false));

        let producer = Producer {
            message_buffer: message_buffer.clone(),
            read_index: read_index.clone(),
            write_index: write_index.clone(),
            producer_counter: producer_counter.clone(),
            consumer_counter: consumer_counter.clone(),
            synchronizer: synchronizer.clone(),
            _marker: PhantomData,
        };

        let consumer = Consumer {
            message_buffer: message_buffer.clone(),
            read_index: read_index.clone(),
            write_index: write_index.clone(),
            producer_counter: producer_counter.clone(),
            consumer_counter: consumer_counter.clone(),
            synchronizer: synchronizer.clone(),
            _marker: PhantomData,
        };

        SPSC { producer, consumer }
    }
}

impl<T: Send> Producer<T> {
    pub fn send(&self, val: T) -> Result<(), SendError<T>> {
        if self.consumer_counter.load(Ordering::SeqCst) == 0 {
            return Err(SendError(val));
        }

        loop {
            while(self.synchronizer.swap(true, Ordering::SeqCst)){}
            let write_index: usize = self.write_index.load(Ordering::SeqCst);
            let read_index: usize = self.read_index.load(Ordering::SeqCst);

            // The write index must not 'overtake' the read index
            // when wrapping around the buffer
            //
            // Since we only have one producer, we do not need an atomic swap
            // to synchronize the write_index increment
            //
            // If the read_index changes during the load, it is okay because
            // the consumer will only read the message when the read index
            // is smaller than the write index
            //
            // Initially, the read index and write index are 0,
            // so we allow a write to the first element of the buffer
            if write_index >= read_index && write_index < read_index + BUFFER_SIZE {
                unsafe {
                    self.message_buffer[write_index % BUFFER_SIZE]
                        .get()
                        .write(Some(val));
                }

                self.write_index.fetch_add(1, Ordering::SeqCst);

                self.synchronizer.swap(false, Ordering::SeqCst);
                return Ok(());
            }
            self.synchronizer.swap(false, Ordering::SeqCst);
        }
    }
}

impl<T: Send> Consumer<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        loop {
            while(self.synchronizer.swap(true, Ordering::SeqCst)){}
            let write_index: usize = self.write_index.load(Ordering::SeqCst);
            let read_index: usize = self.read_index.load(Ordering::SeqCst);

            // When no producer is active and the consumer read all messages, we are done
            if read_index == write_index && self.producer_counter.load(Ordering::SeqCst) == 0 {
                self.synchronizer.swap(false, Ordering::SeqCst);
                return Err(RecvError);
            }

            // since there is only one consumer, we do not need an atomic swap
            // to synchronize the read_index increment
            //
            // If the write_index changes during the load, it is okay because
            // the write index will always be greater than the read index and the
            // producer ensures, that the write index never overtakes the read index
            // when wrapping around the buffer
            if read_index < write_index {
                unsafe {
                    let val = self.message_buffer[read_index % BUFFER_SIZE]
                        .get()
                        .replace(None);
                    self.read_index.fetch_add(1, Ordering::SeqCst);
                    self.synchronizer.swap(false, Ordering::SeqCst);
                    return Ok(val.unwrap());
                }
            }
            self.synchronizer.swap(false, Ordering::SeqCst);
        }
    }
}

impl<T: Send> Iterator for Consumer<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        // TODO: fill with life
        unimplemented!()
    }
}

unsafe impl<T: Send> Send for Producer<T> {}
unsafe impl<T: Send> Send for Consumer<T> {}

impl<T: Send> Drop for Producer<T> {
    fn drop(&mut self) {
        self.producer_counter.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<T: Send> Drop for Consumer<T> {
    fn drop(&mut self) {
        self.consumer_counter.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn channel<T: Send>() -> (Producer<T>, Consumer<T>) {
    let spsc: SPSC<T> = SPSC::new();
    return (spsc.producer, spsc.consumer);
}

// vorimplementierte Testsuite; bei Bedarf erweitern!

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;
    use std::collections::HashSet;
    use std::sync::Mutex;
    use std::thread;

    use super::*;

    lazy_static! {
        static ref FOO_SET: Mutex<HashSet<i32>> = Mutex::new(HashSet::new());
    }

    #[derive(Debug)]
    struct Foo(i32);

    impl Foo {
        fn new(key: i32) -> Self {
            assert!(
                FOO_SET.lock().unwrap().insert(key),
                "double initialisation of element {}",
                key
            );
            Foo(key)
        }
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            assert!(
                FOO_SET.lock().unwrap().remove(&self.0),
                "double free of element {}",
                self.0
            );
        }
    }

    // range of elements to be moved across the channel during testing
    const ELEMS: std::ops::Range<i32> = 0..1000;

    #[test]
    fn unused_elements_are_dropped() {
        lazy_static::initialize(&FOO_SET);

        for i in 0..100 {
            println!("Thread {} ", i);
            let (px, cx) = channel();
            let handle = thread::spawn(move || {
                for i in 0.. {
                    if px.send(Foo::new(i)).is_err() {
                        println!("AHHHHH on i: {}", i);
                        return;
                    }
                }
            });

            for _ in 0..i {
                cx.recv().unwrap();
            }

            drop(cx);

            assert!(handle.join().is_ok());

            let map = FOO_SET.lock().unwrap();
            if !map.is_empty() {
                panic!("FOO_MAP not empty: {:?}", *map);
            }
        }
    }

    #[test]
    fn elements_arrive_ordered() {
        let (px, cx) = channel();

        thread::spawn(move || {
            for i in ELEMS {
                px.send(i).unwrap();
            }
        });

        for i in ELEMS {
            assert_eq!(i, cx.recv().unwrap());
        }

        assert!(cx.recv().is_err());
    }

    #[test]
    fn all_elements_arrive() {
        for _ in 0..100 {
            let (px, cx) = channel();
            let handle = thread::spawn(move || {
                let mut count = 0;

                while let Ok(_) = cx.recv() {
                    count += 1;
                }

                count
            });

            thread::spawn(move || {
                for i in ELEMS {
                    px.send(i).unwrap();
                }
            });

            match handle.join() {
                Ok(count) => assert_eq!(count, ELEMS.len()),
                Err(_) => panic!("Error: join() returned Err"),
            }
        }
    }
}
