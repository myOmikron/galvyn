# Forum Server

An example of how one could combine `rorm` with `axum` to implement a simple forum

It is by no means "complete" and not intended be used as an inspiration for how to use `axum`.

## Usage

Create the migrations
`cargo run --package example-forum-server -- make-migrations migrations/`

Apply them
`cargo run --package example-forum-server -- migrate migrations/`

Run the server
`cargo run --package example-forum-server -- start`

(Optional) Run a test client
`cargo run --package example-forum-server -- test`

## CI

This project is built and run in the CI.

It stays up to date with the current version of `rorm` and serves as a test for `rorm`.