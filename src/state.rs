use primitives::{Error, Positioned, RangeStream, StreamOnce};

/// Trait for tracking the current position of a `Stream`.
pub trait Positioner<Item> {
    /// The type which keeps track of the position
    type Position: Clone + Ord;
    /// Returns the current position
    fn position(&self) -> Self::Position;
    /// Updates the position given that `item` has been taken from the stream
    fn update(&mut self, item: &Item);
}

/// Trait for tracking the current position of a `RangeStream`.
pub trait RangePositioner<Item, Range>: Positioner<Item> {
    /// Updates the position given that `range` has been taken from the stream
    fn update_range(&mut self, range: &Range);
}

/// The `State<I>` struct maintains the current position in the stream `I` using
/// the `Positioner` trait to track the position.
///
/// ```
/// # extern crate combine;
/// # use combine::{token, Parser, ParseError};
/// # use combine::primitives::{Error};
/// # use combine::state::{State, IndexPositioner};
/// # fn main() {
///     let result = token(b'9')
///         .message("Not a nine")
///         .parse(State::new(&b"8"[..], IndexPositioner::new()));
///     assert_eq!(result, Err(ParseError {
///         position: 0,
///         errors: vec![
///             Error::Unexpected(b'8'.into()),
///             Error::Expected(b'9'.into()),
///             Error::Message("Not a nine".into())
///         ]
///     }));
/// # }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct State<I, X> {
    /// The input stream used when items are requested
    pub input: I,
    /// The positioner used to update the current position
    pub positioner: X,
}

impl<I, X> State<I, X>
where
    I: StreamOnce,
    X: Positioner<I::Item>,
{
    /// Creates a new `State<I, X>` from an input stream and a positioner.
    pub fn new(input: I, positioner: X) -> State<I, X> {
        State {
            input: input,
            positioner: positioner,
        }
    }
}

impl<I, X> Positioned for State<I, X>
where
    I: StreamOnce,
    X: Positioner<I::Item>,
{
    type Position = X::Position;

    #[inline(always)]
    fn position(&self) -> Self::Position {
        self.positioner.position()
    }
}

impl<I, X> StreamOnce for State<I, X>
where
    I: StreamOnce,
    X: Positioner<I::Item>,
{
    type Item = I::Item;
    type Range = I::Range;

    #[inline]
    fn uncons(&mut self) -> Result<I::Item, Error<I::Item, I::Range>> {
        self.input.uncons().map(|c| {
            self.positioner.update(&c);
            c
        })
    }
}

/// The `IndexPositioner<Item, Range>` struct maintains the current index into the stream `I`.  The
/// initial index is index 0.  Each `Item` consumed increments the index by 1; each `range` consumed
/// increments the position by `range.len()`.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexPositioner(usize);

impl<Item> Positioner<Item> for IndexPositioner
where
    Item: PartialEq + Clone,
{
    type Position = usize;

    #[inline(always)]
    fn position(&self) -> usize {
        self.0
    }

    #[inline]
    fn update(&mut self, _item: &Item) {
        self.0 += 1
    }
}

impl IndexPositioner {
    pub fn new() -> IndexPositioner {
        IndexPositioner::new_with_position(0)
    }

    pub fn new_with_position(position: usize) -> IndexPositioner {
        IndexPositioner(position)
    }
}

impl<Item, Range> RangePositioner<Item, Range> for IndexPositioner
where
    Item: PartialEq + Clone,
    Range: PartialEq + Clone + ::primitives::Range,
{
    fn update_range(&mut self, range: &Range) {
        self.0 += range.len()
    }
}

impl<I, X> RangeStream for State<I, X>
where
    I: RangeStream,
    X: Clone + RangePositioner<I::Item, I::Range>,
    I::Position: Clone + Ord,
{
    #[inline]
    fn uncons_range(&mut self, size: usize) -> Result<I::Range, Error<I::Item, I::Range>> {
        self.input.uncons_range(size).map(|range| {
            self.positioner.update_range(&range);
            range
        })
    }

    #[inline]
    fn uncons_while<F>(&mut self, mut predicate: F) -> Result<I::Range, Error<I::Item, I::Range>>
    where
        F: FnMut(I::Item) -> bool,
    {
        let positioner = &mut self.positioner;
        self.input.uncons_while(|t| if predicate(t.clone()) {
            positioner.update(&t);
            true
        } else {
            false
        })
    }

    #[inline]
    fn distance(&self, end: &Self) -> usize {
        self.input.distance(&end.input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitives::Parser;

    #[test]
    fn test_positioner() {
        let input = ["a".to_string(), "b".to_string()];
        let mut parser = ::any();
        let result = parser.parse(State::new(&input[..], IndexPositioner::new()));
        assert_eq!(
            result,
            Ok((
                "a".to_string(),
                State::new(
                    &["b".to_string()][..],
                    IndexPositioner::new_with_position(1)
                )
            ))
        );
    }

    #[test]
    fn test_range_positioner() {
        let input = ["a".to_string(), "b".to_string(), "c".to_string()];
        let mut parser = ::range::take(2);
        let result = parser.parse(State::new(&input[..], IndexPositioner::new()));
        assert_eq!(
            result,
            Ok((
                &["a".to_string(), "b".to_string()][..],
                State::new(
                    &["c".to_string()][..],
                    IndexPositioner::new_with_position(2)
                )
            ))
        );
    }
}
