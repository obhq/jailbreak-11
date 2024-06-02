# Jailbreak for 11.00

This is an experimental jailbreak for PS4 11.00 or lower based on TheFloW proof-of-concept [exploit](https://github.com/TheOfficialFloW/PPPwn). **This jailbreak is under development and does not working yet**.

## Requirements

Same as TheFloW proof-of-concept, except Python and GCC are not required.

## Setup

You need to connect the PS4 and the computer with an Ethernet cable **without** any Ethernet switch in the middle.

## Running

Run the following command on the computer that connected with the PS4 to find the index of connected port:

```sh
ip link
```

It will output something like:

```
1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN mode DEFAULT group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
2: enp3s0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP mode DEFAULT group default qlen 1000
    link/ether ??:??:??:??:??:?? brd ff:ff:ff:ff:ff:ff
```

My computer has only one Ethernet so `2` is an obvious index for me. Once figure out run the following command:

```sh
sudo ./jailbreak-11 INDEX
```

Replace `INDEX` with the Ethernet index then open the PS4 and go to `Settings > Network > Set Up Internet Connection > Use a LAN Cable > Custom > PPPoE`. Enter a random `PPPoE User ID` and `PPPoE Password`.

## Building from source

### Prerequisites

- Rust on the latest stable channel

### Install additional Rust target

```sh
rustup target add x86_64-unknown-none
```

### Install additional tools

```sh
cargo install cargo-binutils
```

```sh
rustup component add llvm-tools
```

### Build the payload

```sh
cargo objcopy -p payload --target x86_64-unknown-none --release -- -O binary payload.bin
```

### Build the jailbreak

```sh
cargo build
```

After this you can follow the instructions on the Running section by changing the `./jailbreak-11` to `./target/debug/jailbreak-11`.

## License

MIT
