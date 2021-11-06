# Automata-Common
Automata Common Libraries

## Build

On Ubuntu/Debian (or similar distributions on [WSL](https://docs.microsoft.com/en-us/windows/wsl/about)), install the following packages:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config llvm-dev libclang-dev clang libssl-dev curl
```

Install Rust through [rustup.rs](https://rustup.rs):

```bash
curl https://getsubstrate.io -sSf | bash -s -- --fast
```

Use the following command to build the code:
```bash
cargo build --release
```