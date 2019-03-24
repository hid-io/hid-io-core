export RUST_LOG=""
stty_orig=`stty -g`
stty -echo -icanon min 1 time 0
cargo run --example rpc
stty $stty_orig
