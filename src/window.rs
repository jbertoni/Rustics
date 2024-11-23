//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//!
//! ## Type
//!
//! * Window\<T\>
//!   * This struct is used internally.  Most users will not use
//!     it directly.
//!
//!   * The Window type implements a set of instances of type T.
//!
//!   * The set has a configurable size limit.
//!
//!   * When a new element of type T is pushed into the window,
//!     the oldest element is deleted if the size limit has been
//!     reached.
//!
//!   * The windows code provides the concept of "live" entries,
//!     which are the newest k items, for a configurable limit k.
//!
//!   * The interface proves iterators to examine the contents
//!     of the window, as well as indexing functions.
//!
//! ## Example
//!```
//!     use rustics::window::Window;
//!     use std::cmp::min;
//!
//!     let     size_limit  = 96;
//!     let     live_limit  = 32;
//!     let mut window      = Window::<usize>::new(size_limit, live_limit);
//!
//!     // Fill the window while checking that the results match
//!     // expectations.
//!
//!     for _i in window.iter_all() {
//!         panic!("iterator_test:  The window should be empty.");
//!     }
//!
//!     for _i in window.iter_live() {
//!         panic!("iterator_test:  The window should be empty.");
//!     }
//!
//!     // First, just fill the array.
//!
//!     for i in 1..=size_limit {
//!         println!("iterator_test:  at {} filling the window.", i);
//!
//!         window.push(i);
//!
//!         // Do a sanity check.
//!
//!         assert!(window.live_len() == min(i, live_limit));
//!         assert!(window.all_len()  == i);
//!     }
//!
//!     // Demo the indexing functions.
//!
//!     let first = window.index_all(0).unwrap();
//!     assert!(*first == 1);
//!
//!     let first_live = window.index_live(0).unwrap();
//!     assert!(*first_live == size_limit - live_limit + 1);
//!
//!     // Check the contents a bit...
//!
//!     let mut i = 1;
//!
//!     for value in window.iter_all() {
//!         assert!(*value == i);
//!
//!         i += 1;
//!     }
//!
//!     // Now keep pushing, and make sure that old elements disappear.
//!
//!     for i in 0..size_limit {
//!         let next_data = i + size_limit;
//!
//!         window.push(next_data);
//!
//!         assert!(window.live_len() == live_limit);
//!         assert!(window.all_len()  == size_limit);
//!     }
//!```

//
// A window contains at most "size_limit" items.  The window also
// supports the concept of "live" entries, which are the last
// "live_limit" entries pushed onto the window.  When the window
// is full and a new item is pushed, the oldest item is dropped.
// Thus, this type can be thought of as a window into the
// last "size_limit" events pushed into the window.
//

/// The Window struct is used internally and is of interest primarily
/// to developers creating a new statistics type.
///
/// A Window instance maintains a set of items of type T.  The set size
/// is limited to a configurable parameter.  The oldest item is dropped
/// when  a new item is entered and the window is full.

#[derive(Clone)]
pub struct Window<T> {
    size_limit:     usize,
    live_limit:     usize,

    current_index:  usize,
    data:           Vec<T>,
}

// The Window type supports scans of all live entries and of
// the entire contents of the window.

/// Defines the sets that can be traversed.

pub enum ScanType {
    Live,
    All,
}

impl<T> Window<T> {
    /// Constructs a new window instance.

    pub fn new(size_limit: usize, live_limit: usize) -> Window<T> {
        if size_limit == 0 {
            panic!("Window:  The size limit must be positive");
        }

        if live_limit > size_limit {
            panic!("Window:  The live limit may not exceed the size limit.");
        }

        let data           = Vec::<T>::with_capacity(size_limit);
        let current_index  = 0;

        Window { size_limit, live_limit, current_index, data }
    }

    /// Adds a new entry to the window.

    pub fn push(&mut self, data:  T) {
        // If this is the first entry, set the "current_index"
        // index to level 1, since "current_index" always points
        // to the oldest entry or the next empty slot, if the
        // window is not yet full.
        //
        // If the window still is not full, push this element
        // and increment "current_index".
        //
        // If the window is full, overwrite the oldest entry
        // and increment current.
        //
        // In all cases, "current_index" wraps back to zero when
        // it reaches the size limit of the queue.

        if self.data.is_empty() {
            self.data.push(data);

            self.current_index = 1;
        } else if self.data.len() < self.size_limit {
            self.data.push(data);

            self.current_index += 1;
        } else {
            self.data[self.current_index] = data;

            self.current_index += 1;
        }

        if self.current_index >= self.size_limit {
            self.current_index = 0;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // Return the index to the oldest live entry.
    //
    // Compute the index to the newest entry from "current_index".
    // Just subtract one from "current_index" one unless we're at
    // the start of the array, then we need to wrap.

    fn index_newest(&self) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let result =
            if self.data.len() < self.size_limit {
                self.data.len() - 1
            } else if self.current_index > 0 {
                self.current_index - 1
            } else {
                self.data.len() - 1
            };

        Some(result)
    }

    /// Returns a pointer to the newest item, if there is one.

    pub fn newest(&self) -> Option<&T> {
        if self.data.is_empty() {
            return None;
        }

        let index_newest = self.index_newest().unwrap();
        Some(&self.data[index_newest])
    }

    /// Returns a mutable reference to the current instance.

    pub fn newest_mut(&mut self) -> Option<&mut T> {
        let index_newest = self.index_newest()?;

        Some(&mut self.data[index_newest])
    }

    /// Returns the number of elements in the set.

    pub fn all_len(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of live elements.

    pub fn live_len(&self) -> usize {
        let mut result = self.data.len();

        if result > self.live_limit {
            result = self.live_limit
        }

        result
    }

    /// Returns a pointer to a given element in the window.
    /// The array is indexed with element zero being the
    /// oldest.

    pub fn index_all(&self, index: usize) -> Option<&T> {
        if index >= self.size_limit {
            panic!("Window::index_all:  That index is too large");
        }

        if index >= self.data.len() {
            return None;
        }

        let mut internal_index =
            if self.data.len() < self.size_limit {
                index
            } else {
                self.current_index + index
            };

        if internal_index >= self.size_limit {
            internal_index -= self.size_limit;
        }

        assert!(internal_index < self.data.len());
        Some(&self.data[internal_index])
    }

    /// Returns a pointer to a live element.  The items are
    /// ordered by age wth the oldest at index 0.

    pub fn index_live(&self, index: usize) -> Option<&T> {
        if index >= self.live_limit {
            return None;
        }

        if index >= self.data.len() {
            return None;
        }

        // Compute how many old entries that aren't live might
        // exist.

        let retain = self.size_limit - self.live_limit;

        let mut internal_index =
            if self.data.len() <= self.live_limit {
                index
            } else if self.data.len() < self.size_limit {
                index + self.current_index - self.live_limit
            } else {
                self.current_index + index + retain
            };

        if internal_index >= self.size_limit {
            internal_index -= self.size_limit;
        }

        assert!(internal_index < self.data.len());

        Some(&self.data[internal_index])
    }

    // Returns a read-only reference to the data, the index of the oldest
    // element, and the index to the oldest live element.  This is for
    // testing only.

    #[cfg(test)]
    fn data(&self, verbose: bool) -> (&Vec<T>, usize, usize) {
        let oldest =
            if self.data.len() < self.size_limit {
                0
            } else {
                assert!(self.data.len() == self.size_limit);
                self.current_index
            };

        let retain_limit = self.size_limit - self.live_limit;

        if verbose {
            println!("HierInteger::data:  len {}, size_limit {}, live_limit {}, current_index {}",
                self.data.len(),
                self.size_limit,
                self.live_limit,
                self.current_index
            );
        }

        let oldest_live =
            if self.data.len() < self.live_limit {
                0
            } else if self.data.len() < self.size_limit {
                self.data.len() - self.live_limit
            } else {
                assert!(self.data.len() == self.size_limit);

                let mut result = self.current_index + retain_limit;

                if result >= self.size_limit {
                    result -= self.size_limit
                }

                result
            };

        assert!(self.data.len() == 0 || oldest      < self.data.len());
        assert!(self.data.len() == 0 || oldest_live < self.data.len());

        (&self.data, oldest, oldest_live)
    }

    /// Deletes all data from the window.  This puts it back into
    /// its initial state.

    pub fn clear(&mut self) {
        self.current_index = 0;
        self.data.clear();
    }

    /// Iterates over all the items in the window.

    pub fn iter_all(&self) -> WindowIterator<T> {
        WindowIterator::<T>::new(self, ScanType::All)
    }

    /// Iterates over all the live items in the window.

    pub fn iter_live(&self) -> WindowIterator<T> {
        WindowIterator::<T>::new(self, ScanType::Live)
    }
}

/// Implements the iterator for the contents of a window.

pub struct WindowIterator<'a, T> {
    window:     &'a Window<T>,
    index:      usize,
    remaining:  usize,
}

impl<'a, T> WindowIterator<'a, T> {
    pub fn new(window: &'a Window<T>, scan_type: ScanType) -> WindowIterator<T> {
        assert!(window.data.len() <= window.size_limit);

        if window.data.is_empty() {
            return WindowIterator { window, index: 0, remaining: 0 };
        }

        let retain_limit = window.size_limit - window.live_limit;

        let scan_limit =
            match scan_type {
                ScanType::All  => { window.size_limit }
                ScanType::Live => { window.live_limit }
            };

        // Check on the number of elements in the scan.

        let window_not_full = window.data.len() < window.size_limit;
        let partial_scan    = scan_limit >= window.data.len();

        // If the amount of data in the window is less than the
        // scan limit, the "remaining" count must be adjust.

        let remaining =
            if partial_scan {
                window.data.len()
            } else {
                scan_limit
            };

        // Now compute the first index.

        let index =
            match scan_type {
                ScanType::All  => {
                    if window_not_full {
                        0
                    } else {
                        window.current_index
                    }
                }
                ScanType::Live => {
                    if partial_scan {
                        0
                    } else if window_not_full {
                        window.all_len() - scan_limit
                    } else {
                        let mut result = window.current_index + retain_limit;

                        if result >= window.all_len() {
                            result -= window.all_len();
                        }

                        result
                    }
                }
            };

        assert!(index < window.data.len());

        WindowIterator { window, index, remaining }
    }

    fn find_next_index(&mut self) -> usize {
        if self.remaining == 0 {
            panic!("WindowIterator::find_next_index:  The iteration has been completed.");
        }

        // Is this the first call for this iterator?  If not, the
        // index has been computed, and we just need to return it,
        // and increment the index.

        let result = self.index;

        self.index += 1;

        if self.index >= self.window.data.len() {
            self.index = 0
        }

        result
    }
}

// Implement the actual iterator trait.

impl<'a, T> Iterator for WindowIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.remaining == 0 {
            return None;
        }

        let index  = self.find_next_index();
        let result = Some(&self.window.data[index]);

        self.remaining -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn simple_window_test(verbose:  bool) {
        println!("simple_window_test:  Start the simple window test.");

        let     window_size = 32;
        let mut window      = Window::<usize>::new(window_size, window_size);

        assert!(window.all_len()  == 0);
        assert!(window.live_len() == 0);

        for i in 0..window_size {
            let (data, oldest, oldest_live) = window.data(verbose);

            assert!(data.len() == i          );
            assert!(oldest     == 0          );
            assert!(oldest     == oldest_live);

            window.push(i);

            assert!(window.all_len()  == i + 1);
            assert!(window.live_len() == i + 1);
            assert!(window.data.len() == i + 1);

            assert!(*window.newest().unwrap() == i);
        }

        // Always be verbose here so that the code is tested.

        let (data, oldest, oldest_live) = window.data(true);

        assert!(data.len() == window_size);
        assert!(oldest     == 0          );
        assert!(oldest     == oldest_live);

        for i in 0..window_size {
            assert!(data[i] == i);
        }

        window.clear();

        let (data, oldest, oldest_live) = window.data(true);

        assert!(data.len()  == 0);
        assert!(oldest      == 0);
        assert!(oldest_live == 0);

        // Make a new window and test the overwrite case.

        let     live_limit   = window_size / 2;
        let     retain_limit = window_size - live_limit;
        let mut window       = Window::<usize>::new(window_size, live_limit);

        println!("simple_window_test:  setup overwrite: window_size {}, live_limit {}",
            window_size,
            live_limit
        );

        // First, fill the window.

        for i in 0..window_size {
            window.push(i);

            assert!(window.all_len() == i + 1);

            if i < live_limit {
                assert!(window.live_len() == i + 1);
            } else {
                assert!(window.live_len() == live_limit);
            }

            let (data, oldest, oldest_live) = window.data(verbose);

            if verbose {
                println!("simple_window_test:  at {} len {}:  all {} -> {}, live {} -> {}",
                    i,
                    data.len(),
                    oldest,
                    data[oldest],
                    oldest_live,
                    data[oldest_live]
                );
            }

            assert!(data.len()    == i + 1);
            assert!(oldest        == 0);
            assert!(data[oldest]  == 0);
            assert!(data[i]       == i);

            if data.len() <= live_limit {
                assert!(oldest_live       == 0);
                assert!(data[oldest_live] == 0);
            } else {
                assert!(oldest_live       == data.len() - live_limit);
                assert!(data[oldest_live] == oldest_live   );
            }
        }

        for i in 0..2 * window_size {
            window.push(i + window_size);

            let (data, oldest, oldest_live) = window.data(verbose);

            if verbose {
                println!("simple_window_test:  push {}, oldest {} -> {}, oldest_live {} -> {}",
                    i, oldest, data[oldest], oldest_live, data[oldest_live]);
             }

            assert!(data.len()        == window_size         );
            assert!(data[oldest]      == i + 1               );
            assert!(data[oldest_live] == i + 1 + retain_limit);
            assert!(window.all_len()  == window_size         );
            assert!(window.live_len() == live_limit          );
        }

        for i in 0..window_size {
            let expect = 2 * window_size + i;

            if verbose {
                println!("simple_window_test:  verify window.data[{}] => {} == {}",
                    i, window.data[i], expect);
            }

            assert!(window.data[i] == expect);
        }


        let mut current_push = 3 * window_size;
        let     oldest_value = window.data[window.current_index];
        let     expect       = oldest_value + window_size;

        println!("simple_window_test:  Start the overwite testing, {} == {} + 31",
            current_push,
            window.data[window.current_index]
        );

        assert!(current_push == expect);

        // Push more values into the window to test the case where we overwrite
        // an existing value.  Check all the values in the array at every iteration.

        for i in 0..2 * window_size {
            let mut current_index = window.current_index;

            if verbose {
                println!("simple_window_test:  at iteration {}, current {}, data {}",
                    i,
                    current_index,
                    window.data[current_index]
                );
            }

            // Check that the window contains what we expect.

            for j in 0..window.data.len() {
                let expect = 2 * window_size + i + j;

                if verbose {
                    println!("simple_window_test:  at [{} {}], data[{}] == {}, expect {}",
                        i,
                        j,
                        current_index,
                        window.data[current_index],
                        expect
                    );
                }

                assert!(window.data[current_index] == expect);

                current_index =
                    if current_index == window.data.len() - 1 {
                        0
                    } else {
                        current_index + 1
                    };
            }

            // Push the next value into the window.

            window.push(current_push);
            current_push += 1;
        }
    }

    fn verify_window(window: &Window<usize>, first_data: usize, verbose: bool) {
        let mut expected = first_data;

        if verbose {
            println!("Verifying the iter_all function, expected oldest {}.", expected);
        }

        // Test the "all" iterator.

        for data in window.iter_all() {
            if verbose {
                println!("verify_window:  oldest {}, expected {}", *data, expected);
            }

            assert!(*data == expected);

            expected += 1;
        }

        let mut expected = first_data;

        // Now test the "all" indexing.

        for i in 0..window.all_len() {
            let data = window.index_all(i).unwrap();

            // For debugging the test.
            //
            //let data =
            //    if let Some(data) = window.index_all(i) {
            //        data
            //    } else {
            //        panic!("verify_window:  All indexing failed at {}", i);
            //    };

            // For debugging the test.
            //
            //if *data != expected {
            //    println!("verify_window:  all iterator at {}, got {}, expected {}",
            //        i, *data, expected);
            //}

            assert!(*data == expected);

            expected += 1;
        }

        let retain_limit = window.size_limit - window.live_limit;

        let first_expected =
            if window.data.len() >= window.size_limit {
                first_data + retain_limit
            } else if window.data.len() <= window.live_limit {
                first_data
            } else {
                first_data + window.data.len() - window.live_limit
            };

        if verbose {
            println!("verify_window:  Verifying the iter_live function, expected oldest live {}.",
                first_expected);
        }

        // Test the live iterator.

        let mut expected = first_expected;

        for data in window.iter_live() {
            if verbose {
                println!("verify_window:  oldest live {}, expected {}", *data, expected);
            }

            assert!(*data == expected);

            expected += 1;
        }

        // Now test the live indexing function.

        let mut expected = first_expected;

        for i in 0..window.live_len() {
            let data = window.index_live(i).unwrap();

            // For debugging the test.
            //
            //let data =
            //    if let Some(data) = window.index_live(i) {
            //        data
            //    } else {
            //        panic!("verify_window:  Live indexing failed at {}", i);
            //    };

            //if *data != expected {
            //    println!("verify_window:  index_live(i) = {}, expected {}",
            //        *data, expected);
            //}

            assert!(*data == expected);

            expected += 1;
        }
    }

    // This test is redundant since the simple_window_test() function now
    // checks all the iterators, but it is a simple example of how to
    // use a window.

    fn sample_usage(verbose: bool) {
        let     size_limit  = 96;
        let     live_limit  = 32;
        let mut window      = Window::<usize>::new(size_limit, live_limit);

        assert!(window.is_empty());

        // First, just fill the array.

        for i in 0..size_limit {
            if verbose {
                println!("iterator_test:  at {} filling the window.", i);
            }

            window.push(i);

            // Let verify_window run the iterators.

            if i == 0 {
                verify_window(&window, 0, true);
            } else {
                verify_window(&window, 0, verbose);
            }
        }

        // Now keep pushing, and make sure that old elements disappear.

        for i in 0..size_limit {
            let next_data = i + size_limit;

            window.push(next_data);

            // Let verify_window run the iterators.

            verify_window(&window, next_data - size_limit + 1, verbose);
        }
    }

    #[test]
    #[should_panic]
    fn test_size_limit() {
        let _ = Window::<usize>::new(0, 100);
    }

    #[test]
    #[should_panic]
    fn test_empty_index_newest() {
        let window = Window::<usize>::new(200, 100);
        let _      = window.index_newest().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_index_all_limit() {
        let size_limit = 200;
        let window     = Window::<usize>::new(size_limit, size_limit / 2);

        let _  = window.index_all(size_limit + 1);
    }

    #[test]
    #[should_panic]
    fn test_index_all_size() {
        let size_limit = 200;
        let window     = Window::<usize>::new(size_limit, size_limit / 2);
        let _          = window.index_all(1).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_index_live_size() {
        let size_limit = 200;
        let window     = Window::<usize>::new(size_limit, size_limit / 2);
        let _          = window.index_live(1).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_index_live_limit() {
        let size_limit = 200;
        let window     = Window::<usize>::new(size_limit, size_limit / 2);
        let _          = window.index_live(size_limit).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_empty_newest() {
        let window = Window::<usize>::new(200, 100);
        let _      = window.newest().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_empty_newest_mut() {
        let mut window = Window::<usize>::new(200, 100);
        let     _      = window.newest_mut().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_bad_live_limit() {
        let _ = Window::<usize>::new(50, 100);
    }

    #[test]
    #[should_panic]
    fn test_empty_all_iter() {
        let     window = Window::<usize>::new(200, 100);
        let mut iter   = window.iter_all();
        let     _      = iter.next().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_empty_live_iter() {
        let     window = Window::<usize>::new(200, 100);
        let mut iter   = window.iter_live();
        let     _      = iter.next().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iterator() {
        let     window = Window::<usize>::new(200, 100);
        let mut iter   = window.iter_all();

        // An empty iterator should panic on the second query.

        let _ = iter.find_next_index();
        let _ = iter.find_next_index();
    }

    fn test_small_window() {
        let mut window = Window::<usize>::new(1,1);
        let     limit  = 20;

        for i in 1..=limit {
            window.push(i);

            let newest = window.newest().unwrap();
            assert!(*newest == i);
            assert!(window.all_len() == 1);
            assert!(window.data.len() == 1);
        }
    }

    #[test]
    fn run_tests() {
        simple_window_test(true);
        sample_usage(true);
        test_small_window();
    }
}
