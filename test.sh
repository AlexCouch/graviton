#!/usr/bin/env sh

cargo run test/1.grav
cargo run test/2.grav
cargo run test/3.grav
echo "14" | cargo run examples/fib.grav