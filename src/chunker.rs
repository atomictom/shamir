use std::clone::Clone;
use std::iter;
use std::iter::Iterator;
use std::marker::Sized;

pub struct Chunker<I: Iterator> {
    iter: I,
    chunk_size: usize,
}

pub struct ChunkerDefault<I: Iterator> {
    iter: I,
    chunk_size: usize,
    default: <I>::Item,
}

pub trait ChunkerExt: Iterator {
    fn chunked(self, chunk_size: usize) -> Chunker<Self>
    where
        Self: Sized,
    {
        return Chunker {
            iter: self,
            chunk_size: chunk_size,
        };
    }

    fn chunked_with_default(self, chunk_size: usize, default: Self::Item) -> ChunkerDefault<Self>
    where
        Self::Item: Clone,
        Self: Sized,
    {
        return ChunkerDefault {
            iter: self,
            chunk_size: chunk_size,
            default: default,
        };
    }
}

impl<I: Iterator> Iterator for Chunker<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret: Vec<I::Item> = Vec::with_capacity(self.chunk_size);
        while let Some(x) = self.iter.next() {
            ret.push(x);
            if ret.len() == self.chunk_size {
                break;
            }
        }
        if ret.len() == 0 {
            return None;
        }

        return Some(ret);
    }
}

impl<I: Iterator> Iterator for ChunkerDefault<I>
where
    I::Item: Clone,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ret: Vec<I::Item> = Vec::with_capacity(self.chunk_size);
        while let Some(x) = self.iter.next() {
            ret.push(x);
            if ret.len() == self.chunk_size {
                break;
            }
        }
        if ret.len() == 0 {
            return None;
        }
        if ret.len() < self.chunk_size {
            ret.extend(iter::repeat(self.default.clone()).take(self.chunk_size - ret.len()));
        }

        return Some(ret);
    }
}

impl<I: Iterator> ChunkerExt for I {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_even_split_default() {
        let bytes = &[1, 2, 3, 4, 5, 6, 7, 8];
        for chunk in bytes.iter().chunked_with_default(2, &0) {
            assert_eq!(chunk.len(), 2);
        }
    }

    #[test]
    fn chunk_even_split_no_default() {
        let bytes = &[1, 2, 3, 4, 5, 6, 7, 8];
        for chunk in bytes.iter().chunked(2) {
            assert_eq!(chunk.len(), 2);
        }
    }

    #[test]
    fn chunk_uneven_split_default() {
        let bytes = &[1, 2, 3, 4, 5, 6, 7, 8];
        for (index, chunk) in bytes.iter().chunked_with_default(3, &0).enumerate() {
            assert_eq!(chunk.len(), 3);
        }
    }

    #[test]
    fn chunk_uneven_split_no_default() {
        let bytes = &[1, 2, 3, 4, 5, 6, 7, 8];
        for (index, chunk) in bytes.iter().chunked(3).enumerate() {
            if index < 2 {
                assert_eq!(chunk.len(), 3);
            } else {
                assert_eq!(chunk.len(), 2);
            }
        }
    }
}
