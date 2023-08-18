use serde::{Deserialize, Serialize};

// TODO: Redesign parts of it with const generic arrays when serde supports it.
// https://github.com/serde-rs/serde/issues/1937

/**
Round-robin buffer.

# Note

Overwrite old data after writing `size` items.

# Design

```plain
------------------> Items order

+-----------------+
|....P=>..........|
+-----------------+

P: Round Robin position
=> : Next position after push()
```

New items are written at `P` then increment position (or wrap it to 0 if `P >= size`).

*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundRobinBuffer<T: Sized> {
    pos: usize,
    size: usize,
    buffer: Box<[T]>,
}

impl<T> RoundRobinBuffer<T>
where
    T: Sized + Default + Copy,
{
    pub fn new(size: usize, default: T) -> Self {
        Self {
            pos: 0,
            size,
            buffer: vec![default; size].into_boxed_slice(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.buffer[self.pos] = value;
        self.pos = (self.pos + 1) % self.size;
    }

    pub fn iter(&self) -> RoundRobinIterator<T> {
        RoundRobinIterator {
            rrb: self,
            pos: self.pos,
            done: false,
        }
    }
}

/**
Iterator for Round-Robin buffer.

# Design

```plain
------------------> Items order

+-----------------+
|...........PS=>..|
+-----------------+

S: Initial iterator position
P: Iterator position
```

Iterate over the buffer, wrapping once until reaching `P` where in this
case `done` is set, and iterator ends at the following `next()`.

*/
#[derive(Debug, Clone)]
pub struct RoundRobinIterator<'a, T: Sized> {
    /// Round-robin buffer being iterated.
    rrb: &'a RoundRobinBuffer<T>,

    /// Position of the iterator in the buffer.
    pos: usize,

    /// Indicate if the iterator has completed.
    /// We need this as self.rrb.pos contains valid information so we can't rely on `self.pos == self.rrb.pos`
    /// to report end; and we don't want to discard this value.
    done: bool,
}

impl<'a, T: Sized> Iterator for RoundRobinIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let value = &self.rrb.buffer[self.pos];

        self.pos = (self.pos + 1) % self.rrb.size;

        if self.pos == self.rrb.pos {
            self.done = true;
        }

        Some(value)
    }

    /**
      # Design

      ```plain
      * I < B
      +-----------------+
      |.....I.....B.....|
      +-----------------+

      'I' already wrapped arround, so B - I + 1 is remaining count (include element at B).

      * I > B
      +-----------------+
      |.....B.....I.....|
      +-----------------+

      'I' haven't wrapped arround, so size - I + B + 1 (include element at B) is remaining.

      * I = B

      0 if done, 1 otherwise

      I: Buffer position
      B: Iterator position
      ```
    */
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.pos < self.rrb.pos {
            self.rrb.pos - self.pos + 1
        } else if self.pos > self.rrb.pos {
            self.rrb.size - self.pos + self.rrb.pos + 1
        } else if self.done {
            0
        } else {
            1
        };

        (remaining, Some(remaining))
    }
}

#[test]
fn round_robin_test_insert() {
    let mut buffer = RoundRobinBuffer::new(32, f64::NAN);
    assert!(f64::is_nan(*buffer.iter().next().unwrap()));

    // Add 32 elements into the buffer.
    (0..32).for_each(|i| buffer.push(i as f64));

    // Elements should come in the same order (we filled the buffer).
    (0..32).zip(buffer.iter()).for_each(|(reference, val)| {
        assert_eq!(reference, *val as i32);
    });

    // Overwrite all elements.
    (32..64).for_each(|i| buffer.push(i as f64));

    (32..64).zip(buffer.iter()).for_each(|(reference, val)| {
        assert_eq!(reference, *val as i32);
    });
}

#[test]
fn round_robin_test_iter_count() {
    let buffer = RoundRobinBuffer::new(32, f64::NAN);
    assert_eq!(buffer.iter().count(), 32);

    let buffer = RoundRobinBuffer::new(1, f64::NAN);
    assert_eq!(buffer.iter().count(), 1);
}