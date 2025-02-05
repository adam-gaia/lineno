# lineno

Display specific lines or ranges of lines of a file

## Motivation

Suppose you need lines 10 to 15 of file 'file.txt' as part of a greater pipeline.
You can use `head` and `tail` together in with `head -n 15 file.txt | tail -n 6`.

While that's relatively easy, lineno makes it even easier:

```console
$ lineno -f file.txt 10..15
foo
bar
baz
qux
quux
didgeridoo

```

What if we want lines 10 through 15 and line 26? My unix-foo isn't good enough to do that in a single pipeline off the top of my head.
But of course its trivial with lineno:

```console
$ lineno -f file.txt 10..15 26
foo
bar
baz
qux
quux
didgeridoo
supercalifragilisticexpialidocious

```

(My contrived-example-foo has always outpaced my unix-foo)

## Detailed Usage

### Filters

lineno takes an arbitrary number of filters and returns lines matching those filters.

Filters any combo of

- Standalone line numbers, e.g. `10`

```console
$ lineno -f ./tests/foo.txt 2
bar

```

- Ranges, e.g.`1:10`

```console
$ lineno -f ./tests/foo.txt 1:3
foo
bar
baz

```

Both the upper and lower bounds on ranges are *inclusive*.

Ranges may either be specified with a ':' or '..' (`1..10` and `1:10` are equivalent) (like the `head` util)

Omitting the upperbound returns all lines from the lowerbound to the end of the file (`24..`) (like the `tail` util)

```console
$ lineno -f ./tests/rocket.txt 8..
3
2
1
blast off!

```

Likewise, omitting the lowerbound returns all lines from the start of the file to the upperbound

```console
$ lineno -f ./tests/rocket.txt ..4
10
9
8
7

```

Multiple filters may be specified by commas or whitespace. There is no need for lists of filters to be in numerical order

```console
$ lineno -f file.txt 3 2 1 10:12
long
text
innoculous
foo
bar
baz

```

or unique

```console
$ lineno -f file.txt 23 23 23 25
spam
spam
spam
eggs

```

Running lineno without any filters cats the file (outputs the file unmodified)

```console
$ lineno -f ./tests/small.txt
line 1
line 2
line 3

```

which, btw, is the same as an empty range (idk why one would want to do that instead of using `cat`, but it's possible)

```console
$ lineno -f ./tests/small.txt ..
line 1
line 2
line 3

```

If no file is specified, lineno will read from stdin

```ignore - TODO: trycmd test skipped until trycmd supports pipes https://github.com/assert-rs/snapbox/issues/172
$ cat ./tests/small.txt | lineno 3
line 3

```

Oh hey, that last example is a useless use of cat. Lets make it even more useless for fun

```ignore - TODO: trycmd test skipped until trycmd supports pipes https://github.com/assert-rs/snapbox/issues/172
$ lineno -f ./tests/small.txt .. | lineno 3
line 3

```

### Options

- `-f, --file <file>`
  You've already seen the '-f' option to provide an input file:

```console
$ lineno -f ./tests/small.txt
line 1
line 2
line 3

```

- `-n`,`--number`
  The '-n' option displays line numbers along with the line's content:

```console
$ lineno -f ./tests/small.txt -n
1: line 1
2: line 2
3: line 3

```
