# ravenwm

A sleek, hybrid window manager with modern sensibilities.

## Development

To work on `ravenwm`, you'll need a nested X server:

```sh
Xephyr -screen 1024x768 :1 &
```

With that running, you can then run `ravenwm` on that X server:

```sh
cargo build --release
DISPLAY=:1 ./target/release/ravenwm
```
