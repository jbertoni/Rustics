//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

#[derive(Clone)]
pub struct Window<T> {
    size_limit:     usize,
    live_limit:     usize,

    current_index:  usize,
    data:           Vec<T>,
}

pub enum ScanType {
    Live,
    All,
}

impl<T> Window<T> {
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

    pub fn push(&mut self, data:  T) {
        if self.data.is_empty() {
            self.current_index = 1;

            self.data.clear();
            self.data.push(data);
        } else if self.data.len() < self.size_limit {
            self.data.push(data);

            self.current_index += 1;

            if self.current_index >= self.size_limit {
                self.current_index = 0;
            }
        } else {
            self.data[self.current_index] = data;
            self.current_index += 1;

            if self.current_index >= self.data.len() {
                self.current_index = 0;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn index_newest(&self) -> usize {
        assert!(!self.data.is_empty());

        let result =
            if self.data.len() < self.size_limit {
                self.data.len() - 1
            } else if self.current_index > 0 {
                self.current_index - 1
            } else {
                self.data.len() - 1
            };

        assert!(result < self.data.len());
        result
    }

    pub fn newest(&self) -> Option<&T> {
        if self.data.is_empty() {
            return None;
        }

        let index_newest = self.index_newest();
        Some(&self.data[index_newest])
    }

    pub fn newest_mut(&mut self) -> Option<&mut T> {
        if self.data.is_empty() {
            return None;
        }

        let index_newest = self.index_newest();
        Some(&mut self.data[index_newest])
    }

    pub fn all_len(&self) -> usize {
        self.data.len()
    }

    pub fn live_len(&self) -> usize {
        let mut result = self.data.len();

        if result > self.live_limit {
            result = self.live_limit
        }

        result
    }

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

        if internal_index >= self.data.len() {
            println!("index_all:  index_all[{}] = {}, size {}, self.current_index {}, self.size_limit {}",
                index, internal_index, self.data.len(), self.current_index, self.size_limit);
        }

        assert!(internal_index < self.data.len());
        Some(&self.data[internal_index])
    }

    pub fn index_live(&self, index: usize) -> Option<&T> {
        if index >= self.live_limit {
            panic!("Window::index_live:  That index is too large");
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

        if internal_index >= self.data.len() {
            println!("index_live:  index_live[{}] = {}, size {}, current_index {}, size_limit {}, live_limit {}",
                index, internal_index, self.data.len(), self.current_index, self.size_limit, self.live_limit);
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

    pub fn clear(&mut self) {
        self.current_index = 0;
        self.data.clear();
    }

    pub fn iter_all(&self) -> WindowIterator<T> {
        WindowIterator::<T>::new(self, ScanType::All)
    }

    pub fn iter_live(&self) -> WindowIterator<T> {
        WindowIterator::<T>::new(self, ScanType::Live)
    }
}

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

        // Now compute the first index.  Make a first guess.

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
        }

        let (data, oldest, oldest_live) = window.data(verbose);

        assert!(data.len() == window_size);
        assert!(oldest     == 0          );
        assert!(oldest     == oldest_live);

        for i in 0..window_size {
            assert!(data[i] == i);
        }


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
                println!("simple_window_test:  overwrite {} len {}:  oldest {} -> {}, oldest_live {} -> {}",
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
                println!("simple_window_test:  overwrite iteration {}, oldest {}, oldest value {}",
                    i,
                    current_index,
                    window.data[current_index]
                );
            }

            // Check that the window contains what we expect.

            for j in 0..window.data.len() {
                let expect = 2 * window_size + i + j;

                if verbose {
                    println!("simple_window_test:  at iteration [{} {}], data[{}] == {}, expect {}",
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
            let data =
                if let Some(data) = window.index_all(i) {
                    data
                } else {
                    panic!("verify_window:  All indexing failed at {}", i);
                };

            if *data != expected {
                println!("verify_window:  all iterator at {}, got {}, expected {}",
                    i, *data, expected);
            }

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
            let data =
                if let Some(data) = window.index_live(i) {
                    data
                } else {
                    panic!("verify_window:  Live indexing failed at {}", i);
                };

            if *data != expected {
                println!("verify_window:  index_live(i) = {}, expected {}",
                    *data, expected);
            }

            assert!(*data == expected);

            expected += 1;
        }
    }

    // This test is redundant since the simple_window_test routine now
    // checks all the iterators, but it is a simple example of how to
    // use a window.

    fn sample_usage(verbose: bool) {
        let     size_limit  = 96;
        let     live_limit  = 32;
        let mut window      = Window::<usize>::new(size_limit, live_limit);

        // Fill the window while checking that the results match expectations.

        for _i in window.iter_all() {
            panic!("iterator_test:  The window should be empty.");
        }

        for _i in window.iter_live() {
            panic!("iterator_test:  The window should be empty.");
        }

        // First, just fill the array.

        for i in 0..size_limit {
            if verbose {
                println!("iterator_test:  at {} filling the window.", i);
            }

            window.push(i);

            // Let verify_window run the iterators.

            verify_window(&window, 0, verbose);
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
    fn run_tests() {
        simple_window_test(false);
        sample_usage(false);
    }
}
