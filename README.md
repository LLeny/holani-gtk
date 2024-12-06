# Holani-gtk

A GTK4 frontend for the Atari Lynx emulator [Holani](https://github.com/LLeny/holani).

![Holani](/assets/holani.jpg?raw=true "Holani")

## Build

You will need [Rust and its package manager Cargo](https://www.rust-lang.org/) and [GTK4](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation.html). 

```
git clone https://github.com/LLeny/holani-gtk.git
```

Build with:

```
cargo build --release
```

The executable will be in the `target/release/` directory.


## Usage

```
Usage: holani-gtk [OPTIONS]

Options:
  -c, --cartridge <CARTRIDGE>  Cartridge, can be .o or a .lnx file
  -s, --single-instance        Allows only one instance running
  -h, --help                   Print help
  -V, --version                Print version
```


