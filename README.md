# Jailbreak for 11.00

This is an experimental jailbreak for PS4 11.00 or lower using TheFloW disclosed [exploit](https://hackerone.com/reports/2177925). **This jailbreak is under development and does not working yet and may not working at all**.

## System requirements

- Computer with one ethernet port available
- Ethernet cable
- Linux
  - A VM running Linux may not work.

## Setup

You need to connect the PS4 and the computer with an ethernet cable **without** any ethernet switch in the middle.

## Running

Run the following command on the computer that connect with the PS4 to find the index of the ethernet port connected with the PS4:

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

My computer has only one ethernet so `2` is the index of the ethernet port for me. Once figure out run the following command:

```sh
sudo ./jailbreak-11 INDEX
```

Replace `INDEX` with the ethernet index then open the PS4 and go to `Settings > Network > Set Up Internet Connection > Use a LAN Cable > Custom > PPPoE`. Enter a random `PPPoE User ID` and `PPPoE Password`.

## License

MIT
