# Tattle

A crate for reporting errors in compilers.

## Motivation

In compiler development, one would like to report as many user errors in source files at once,
so that the user can choose what order to fix them in rather than having to fix the first one
found first. However, these errors are almost never handled.

Thus, it doesn't make sense to use traditional `Result` enums to communicate these errors.
Rather, errors should either be signaled by `Option<T>` or by including error variants in `T`
itself. However, errors should still be *reported* with as much detail as is reasonable. Tattle
is a crate for doing this reporting.

## Usage

Right now documentation is sparse: see the usage in [fexplib](https://github.com/ToposInstitute/fexplib) for examples.

## Inspirations

This crate is mainly inspired by [asai](https://github.com/RedPRL/asai). However, it also owes
influence to [zig's error codes](https://ziglang.org/documentation/master/#Errors).
