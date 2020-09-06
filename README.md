# Experimental Reed-Solomon and Shamir Secret Sharing in Rust

## Background

This evolved out of an exercise in [A Programmer's Introduction to
Mathematics](https://www.amazon.co.uk/Programmers-Introduction-Mathematics-Dr-Jeremy/dp/1727125452)
from the chapter on Polynomials. The exercise had to do with [Shamir's secret
sharing algorithm](https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing), but
I was familiar enough with Reed-Solomon to realize they were probably based on
the same concepts. I was looking for a programming project while reading that,
so I decided to make a library to do RS encoding, and later actually did the
Shamir's secret sharing portion. Additionally, this was a chance to learn Rust.

Currently the code is quite messy since I used a very slow method of computing
RS at first (Lagrangian interpolation of polynomials) and only later added in a
faster way (using Vandermonde matrices). Because it was a learning exercise, I
kept both approaches.

## Functionality

Right now it's very half baked. There is a pseudo-library for doing RS encoding,
but it's not very easy to use. There are some functions for doing a limited
Shamir setup on top of the RS library. The Shamir setup takes a page from
passphrases / diceware passwords and generates words instead of bytes so it's
easier for humans to work with.

## Future Work

At some point I'd like to refactor the interface to be simpler to use and I'd
like to clean up the code to remove some of the cruft that came from learning
RS.

I'd also like to publish a tool for doing Shamir secret sharing at some point.
