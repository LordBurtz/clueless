use std::cmp::{max, Ord};
use std::fmt::Debug;

/// Represents an interval with a lower and upper bound.
#[derive(Debug, Clone)]
pub struct Interval<T: Ord + Copy + Debug> {
    low: T,
    high: T,
}

impl<T: Ord + Copy + Debug> Interval<T> {
    pub fn new(low: T, high: T) -> Self {
        assert!(low <= high, "Low endpoint must be <= high endpoint.");
        Interval { low, high }
    }

    /// Checks if this interval is fully contained within another interval.
    fn is_contained_in(&self, other: &Interval<T>) -> bool {
        other.low <= self.low && self.high <= other.high
    }
}

/// Node of the interval tree.
#[derive(Debug)]
pub struct IntervalNode<T: Ord + Copy + Debug, D> {
    interval: Interval<T>,
    data: D,
    max: T,
    left: Option<Box<IntervalNode<T, D>>>,
    right: Option<Box<IntervalNode<T, D>>>,
}

impl<T: Ord + Copy + Debug, D> IntervalNode<T, D> {
    fn new(interval: Interval<T>, data: D) -> Self {
        let max = interval.high;
        IntervalNode {
            interval,
            data,
            max,
            left: None,
            right: None,
        }
    }

    /// Updates the `max` value of the node based on its children's `max` values.
    fn update_max(&mut self) {
        self.max = self.interval.high;
        if let Some(ref left) = self.left {
            self.max = max(self.max, left.max);
        }
        if let Some(ref right) = self.right {
            self.max = max(self.max, right.max);
        }
    }

    /// Inserts a new interval and associated data into the subtree.
    fn insert(&mut self, interval: Interval<T>, data: D) {
        if interval.low < self.interval.low {
            if let Some(ref mut left) = self.left {
                left.insert(interval, data);
            } else {
                self.left = Some(Box::new(IntervalNode::new(interval, data)));
            }
        } else {
            if let Some(ref mut right) = self.right {
                right.insert(interval, data);
            } else {
                self.right = Some(Box::new(IntervalNode::new(interval, data)));
            }
        }
        self.update_max();
    }

    /// Queries the subtree for intervals fully contained within the given interval.
    fn query<'a>(&'a self, interval: &Interval<T>, results: &mut Vec<&'a D>) {
        // Check left subtree if needed
        if let Some(ref left) = self.left {
            if left.max >= interval.low {
                left.query(interval, results);
            }
        }

        // Check current interval for containment
        if self.interval.is_contained_in(interval) {
            results.push(&self.data);
        }

        // Check right subtree if needed
        if let Some(ref right) = self.right {
            if self.interval.low <= interval.high {
                right.query(interval, results);
            }
        }
    }
}

/// The interval tree structure.
#[derive(Debug)]
pub struct IntervalTree<T: Ord + Copy + Debug, D> {
    root: Option<Box<IntervalNode<T, D>>>,
}

impl<T: Ord + Copy + Debug, D> IntervalTree<T, D> {
    pub fn new() -> Self {
        IntervalTree { root: None }
    }

    /// Inserts a new interval and associated data into the tree.
    pub fn insert(&mut self, interval: Interval<T>, data: D) {
        if let Some(ref mut root) = self.root {
            root.insert(interval, data);
        } else {
            self.root = Some(Box::new(IntervalNode::new(interval, data)));
        }
    }

    /// Queries the tree and returns an iterator over data fully contained within the given interval.
    pub fn query(&self, interval: &Interval<T>) -> QueryIterator<D> {
        let mut results = Vec::new();
        if let Some(ref root) = self.root {
            root.query(interval, &mut results);
        }
        QueryIterator {
            data_iter: results.into_iter(),
        }
    }

    pub fn clear(&mut self) {
        self.root = None;
    }
}

/// Iterator over the query results.
pub struct QueryIterator<'a, D> {
    data_iter: std::vec::IntoIter<&'a D>,
}

impl<'a, D> Iterator for QueryIterator<'a, D> {
    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        self.data_iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_insertion_and_query_contained() {
        let mut tree = IntervalTree::<u64, &str>::new();

        // Insert intervals with associated data.
        tree.insert(Interval::new(10, 20), "Event A");
        tree.insert(Interval::new(15, 25), "Event B");
        tree.insert(Interval::new(17, 19), "Event C");
        tree.insert(Interval::new(30, 40), "Event D");

        // Query for intervals fully contained within [14, 18].
        let query_interval = Interval::new(14, 18);
        let results: Vec<&str> = tree.query(&query_interval).copied().collect();

        let expected: Vec<&str> = vec![];
        assert_eq!(results, expected);
    }

    #[test]
    fn test_no_contained_intervals() {
        let mut tree = IntervalTree::<i32, &str>::new();

        tree.insert(Interval::new(1, 5), "Interval 1");
        tree.insert(Interval::new(6, 10), "Interval 2");
        tree.insert(Interval::new(11, 15), "Interval 3");

        // Query an interval that does not fully contain any intervals in the tree.
        let query_interval = Interval::new(16, 20);
        let results: Vec<&str> = tree.query(&query_interval).copied().collect();

        assert!(results.is_empty());
    }

    #[test]
    fn test_fully_contained_intervals() {
        let mut tree = IntervalTree::<i32, &str>::new();

        tree.insert(Interval::new(10, 20), "Interval A");
        tree.insert(Interval::new(12, 18), "Interval B");
        tree.insert(Interval::new(15, 25), "Interval C");
        tree.insert(Interval::new(30, 40), "Interval D");

        // Query an interval that fully contains "Interval B" and "Interval C".
        let query_interval = Interval::new(10, 25);
        let results: Vec<&str> = tree.query(&query_interval).copied().collect();

        let mut expected = vec!["Interval A", "Interval B", "Interval C"];
        expected.sort();
        let mut results_sorted = results.clone();
        results_sorted.sort();

        assert_eq!(results_sorted, expected);
    }

    #[test]
    fn test_exact_match() {
        let mut tree = IntervalTree::<i32, &str>::new();

        tree.insert(Interval::new(10, 20), "Interval A");

        // Query an interval that exactly matches "Interval A".
        let query_interval = Interval::new(10, 20);
        let results: Vec<&str> = tree.query(&query_interval).copied().collect();

        assert_eq!(results, vec!["Interval A"]);
    }

    #[test]
    fn test_zero_length_interval_containment() {
        let mut tree = IntervalTree::<i32, &str>::new();

        tree.insert(Interval::new(10, 10), "Zero-Length Interval");

        // Query interval that contains the zero-length interval.
        let query_interval = Interval::new(5, 15);
        let results: Vec<&str> = tree.query(&query_interval).copied().collect();

        assert_eq!(results, vec!["Zero-Length Interval"]);
    }

    #[test]
    fn test_large_interval_contains_smaller_intervals() {
        let mut tree = IntervalTree::<i32, i32>::new();

        // Insert multiple small intervals.
        for i in 0..100 {
            tree.insert(Interval::new(i * 10, i * 10 + 5), i);
        }

        // Query a large interval that should contain some of the small intervals.
        let query_interval = Interval::new(200, 500);
        let results: Vec<&i32> = tree.query(&query_interval).collect();

        // Expected intervals are those where (i*10 >= 200) && (i*10+5 <= 500)
        let expected: Vec<i32> = (20..50).collect();

        let results_values: Vec<i32> = results.into_iter().copied().collect();

        assert_eq!(results_values, expected);
    }
}
