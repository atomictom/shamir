# Experimental Shamir Secret Sharing in Rust

## Background

This evolved out of an exercise in [A Programmer's Introduction to
Mathematics](https://www.amazon.co.uk/Programmers-Introduction-Mathematics-Dr-Jeremy/dp/1727125452)
from the chapter on Polynomials. The exercise had to do with [Shamir's secret
sharing algorithm](https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing), I
was looking for a programming project while reading that, so I decided to make
it. Additionally, this was a chance to learn Rust.

Currently the code is quite messy since I used a very slow method of computing
RS at first (Lagrangian interpolation of polynomials) and only later added in a
faster way (using Vandermonde matrices). Because it was a learning exercise, I
kept both approaches. Additionally, my inexperience with Rust meant that some of
my interfaces were subpar or I had to make compromises to make things compile.
Lastly, the code has been written in fits and bursts often on airplanes or in
the evenings so I wouldn't say it's my highest quality code!

## Functionality

Shamir secret sharing works by generating a set of shards (based on
polynomials), using one of them as a secret, and distributing the other shards.
If some subset of the shards come together, all the shards (including the
secret) can be regenerated. The parameters are the total number of shards to
generate (with one being the secret) and the required number of shards to
regenerate all shards.

Shards are generated as "phrases" which are repeated rounds of polynomial
generation with 8-bit bytes as the underlying symbol. The bytes are transformed
into words based on a list provided with the program to form something like
diceware passphrases. The number of words in the phrase can be controlled by a
flag.

The binary is meant to be used as a CLI tool with flags for the total and
required number of shards and phrase length. When restoring, it is important
that the original parameters are passed back in. However, the first word in each
shard is its index, so shards can be passed back in any order. While there are
no flags for the shards, they can be passed in via stdin as a newline separated
list.

## Demo

```
$ ./shamir generate --total 6 --required 3 --words 10

-- Generating secret and shards... --
Shards: 6, required: 3
Secret: smash putt savor union cola legal body shout draw slaw
Shard 1: affix urban decal sharp mummy stain remix trout zebra haven cleft
Shard 2: agony wafer virus swoop trade cleft tweak query opera crave pep
Shard 3: ajar slaw crop decal crisp puppy tiger mop said clash path
Shard 4: angel zebra daily vowel mop tweak clap risk bring plant old
Shard 5: angle clap fling shout diner baton ashes grid petri path sharp
```

```
$ cat <<END | ./shamir restore --total 6 --required 3 --words 10
affix urban decal sharp mummy stain remix trout zebra haven cleft
angel zebra daily vowel mop tweak clap risk bring plant old
agony wafer virus swoop trade cleft tweak query opera crave pep
END

-- Restoring the secret... --
You will be prompted to enter 3 shards (in any order)...
Input shard 0: Input shard 1: Input shard 2: Valid: [false, true, true, false, true, false, false]
Length: 11
Encoding: Encoding { data_chunks: 3, code_chunks: 4 }
Shards: 3, required: 3
Password: smash putt savor union cola legal body shout draw slaw
```

## Disclaimer

I'm not a security professional and may have made mistakes or have bugs in my
code. I do not recommend this for any serious applications. It's published only
as a cool project for learning (both my learning and that of anyone interested).

The implementation currently uses exponent and log tables to implement
multiplication which is susceptible to side channel attacks (memory timing).
While there is a direct implementation of multiplication (via "Russian Peasant
Multiplication"), there is no direct implementation of division (it's generated
by brute forcing multiplication to find the inverse element).
