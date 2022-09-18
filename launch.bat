echo on

cargo build

pause

start "one" cargo run -- "one"

pause

start "two" cargo run -- "two"

pause

start "three" cargo run -- "three"

pause

del Cargo.lock

rmdir /s/q target

pause
